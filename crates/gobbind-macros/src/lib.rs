use proc_macro_error2::{abort, emit_error, proc_macro_error};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    FnArg, Ident, ImplItem, ImplItemFn, ItemImpl, Pat, ReturnType, Signature, Type,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

/// Generates a `GDExtensionClassCreationInfo` for a type and registers it.
fn expand_class_registration(
    api: &Ident,
    rust_type: &Type,
    class_name_ident: &Ident,
) -> TokenStream {
    quote_spanned! {rust_type.span()=>
        {
            let class_info = crate::__rt::GDExtensionClassCreationInfo {
                is_virtual: false,
                is_abstract: false,
                set_func: ::core::ptr::null(),
                get_func: ::core::ptr::null(),
                get_property_list_func: ::core::ptr::null(),
                free_property_list_func: ::core::ptr::null(),
                property_can_revert_func: ::core::ptr::null(),
                property_get_revert_func: ::core::ptr::null(),
                notification_func: ::core::ptr::null(),
                to_string_func: ::core::ptr::null(),
                reference_func: ::core::ptr::null(),
                unreference_func: ::core::ptr::null(),
                create_instance_func: crate::__rt::create_instance::<#rust_type>, // FIXME
                free_instance_func: crate::__rt::free_instance::<#rust_type>,
                get_virtual_func: ::core::ptr::null(),
                get_rid_func: ::core::ptr::null(),
                class_userdata: ::core::ptr::null_mut(),
            };

            unsafe {
                let parent_class_name = <crate::__rt::GDExtensionString as ::core::str::FromStr>::from_str("Node").unwrap();
                let parent_class_name = crate::__rt::GDExtensionStringName::from(&parent_class_name);

                (#api.classdb_register_extension_class)(
                    #api.library,
                    &raw const #class_name_ident,
                    &raw const parent_class_name,
                    &raw const class_info,
                );
            }
        }
    }
}

fn expand_thunks(call_ident: &Ident, ptrcall_ident: &Ident, signature: &Signature) -> TokenStream {
    let target_fn = &signature.ident;
    let self_conversion = quote! {
        unsafe { &*instance.cast::<Self>() }
    };

    let error_ident = format_ident!("error_out");
    let call_arguments = signature.inputs.iter()
        .enumerate()
        .map(|(arg_idx, arg)| {
            let FnArg::Typed(arg) = arg else {
                return quote_spanned! {arg.span()=> #self_conversion };
            };

            let ty = &arg.ty;
            let tokens = quote_spanned! {arg.span()=>
                unsafe {
                    ::core::assert!(argument_count > (#arg_idx as i64), "tried to call a GDExtension method with too few arguments");
                    let p = *args.add(#arg_idx);
                    let Some(value) = <#ty as crate::__rt::PropertyConvertInternal>::convert_from_variant(p) else {
                        (*#error_ident).error = crate::__rt::GDExtensionCallErrorType::InvalidArgument;
                        (*#error_ident).argument = #arg_idx as i32;
                        return;
                    };
                    value
                }
            };

            tokens
        })
        .collect::<Vec<_>>();

    let ptrcall_arguments = signature
        .inputs
        .iter()
        .enumerate()
        .map(|(arg_idx, arg)| {
            let FnArg::Typed(arg) = arg else {
                return quote_spanned! {arg.span()=> #self_conversion };
            };

            let ty = &arg.ty;
            let tokens = quote_spanned! {arg.span()=>
                unsafe {
                    let p = *args.add(#arg_idx);
                    <#ty as crate::__rt::PropertyConvertInternal>::convert_from_extension_type(p)
                }
            };

            tokens
        })
        .collect::<Vec<_>>();

    let arg_count = signature.inputs.len();
    quote_spanned! {signature.span()=>
        #[allow(unused_variables)]
        #[allow(clippy::let_unit_value)]
        unsafe extern "C" fn #call_ident(
            _method_userdata: *mut ::core::ffi::c_void,
            instance: *const crate::__rt::GDExtensionClassInstance,
            args: *const crate::__rt::GDExtensionVariantPtr,
            argument_count: i64,
            ret: crate::__rt::GDExtensionVariantPtr,
            #error_ident: *mut crate::__rt::GDExtensionCallError,
        ) {
            let mut error = crate::__rt::GDExtensionCallError {
                error: crate::__rt::GDExtensionCallErrorType::OK,
                argument: 0,
                expected: 0,
            };

            if argument_count < (#arg_count as i64) {
                error.error = crate::__rt::GDExtensionCallErrorType::TooFewArguments;
                error.expected = #arg_count as i32;
            } else if argument_count > (#arg_count as i64) {
                error.error = crate::__rt::GDExtensionCallErrorType::TooManyArguments;
                error.expected = #arg_count as i32;
            }

            unsafe { *#error_ident = error };
            if error.error != crate::__rt::GDExtensionCallErrorType::OK {
                return;
            }

            let value = Self::#target_fn(
                #(#call_arguments),*
            );
            unsafe { crate::__rt::PropertyConvertInternal::convert_to_variant(value, ret); }
        }

        #[allow(unused_variables)]
        #[allow(clippy::let_unit_value)]
        unsafe extern "C" fn #ptrcall_ident(
            _method_userdata: *mut ::core::ffi::c_void,
            instance: *const crate::__rt::GDExtensionClassInstance,
            args: *const crate::__rt::GDExtensionTypePtr,
            ret: crate::__rt::GDExtensionTypePtr,
        ) {
            let value = Self::#target_fn(
                #(#ptrcall_arguments),*
            );
            unsafe { crate::__rt::PropertyConvertInternal::convert_to_extension_type(value, ret); }
        }
    }
}

fn expand_method_registration(
    api: &Ident,
    godot_method_name: &str,
    class_name_ident: &Ident,
    signature: &Signature,
    call_ident: &Ident,
    ptrcall_ident: &Ident,
) -> TokenStream {
    let mut argument_properties = Vec::with_capacity(signature.inputs.len());
    let mut argument_metadata = Vec::with_capacity(signature.inputs.len());
    let mut is_static = true;

    for argument in &signature.inputs {
        let FnArg::Typed(pattern_type) = argument else {
            is_static = false;
            continue;
        };

        let argument_ident = match &*pattern_type.pat {
            Pat::Ident(ident) => {
                if ident.subpat.is_some() {
                    emit_error!(ident, "argument bindings are unsupported");
                }

                &ident.ident
            }
            other => {
                emit_error!(other, "argument bindings are unsupported");
                continue;
            }
        };

        let argument_type = &*pattern_type.ty;
        let argument_name = argument_ident.to_string();

        let property = quote_spanned! {argument.span()=>{
            let property_name = <crate::__rt::GDExtensionString as ::core::str::FromStr>::from_str(#argument_name).unwrap();
            let property_name = crate::__rt::GDExtensionStringName::from(&property_name);

            crate::__rt::GDExtensionPropertyInfo {
                r#type: <#argument_type as crate::__rt::PropertyConvertInternal>::as_variant_type(),
                name: &raw const property_name,
                class_name: &raw const empty_string_name,
                hint: crate::__rt::GDExtensionPropertyHint::None,
                hint_string: &raw const empty_string,
                usage: crate::__rt::GDExtensionPropertyUsageFlags::None,
            }
        }};

        let metadata = quote_spanned! {argument.span()=>
            <#argument_type as crate::__rt::PropertyConvertInternal>::as_argument_metadata()
        };

        argument_properties.push(property);
        argument_metadata.push(metadata);
    }

    let (return_value_info, return_value_metadata) =
        if let ReturnType::Type(arrow, ref typ) = signature.output {
            let info = quote_spanned! { arrow.span() => {
                ::core::option::Option::Some(crate::__rt::GDExtensionPropertyInfo {
                    r#type: <#typ as crate::__rt::PropertyConvertInternal>::as_variant_type(),
                    name: &raw const empty_string_name,
                    class_name: &raw const empty_string_name,
                    hint: crate::__rt::GDExtensionPropertyHint::None,
                    hint_string: &raw const empty_string,
                    usage: crate::__rt::GDExtensionPropertyUsageFlags::None,
                })
            }};
            let metadata = quote_spanned! { arrow.span() =>
                <#typ as crate::__rt::PropertyConvertInternal>::as_argument_metadata()
            };
            (info, metadata)
        } else {
            let info = quote! { ::core::option::Option::None };
            let metadata = quote! { crate::__rt::GDExtensionClassMethodArgumentMetadata::None };
            (info, metadata)
        };

    let method_flags = if is_static {
        quote! { crate::__rt::GDExtensionClassMethodFlags::Static }
    } else {
        quote! { crate::__rt::GDExtensionClassMethodFlags::Normal }
    };

    quote_spanned! { signature.span()=> {
        let method_name = <crate::__rt::GDExtensionString as ::core::str::FromStr>::from_str(#godot_method_name).unwrap();
        let method_name = crate::__rt::GDExtensionStringName::from(&method_name);

        let empty_string = <crate::__rt::GDExtensionString as ::core::str::FromStr>::from_str("").unwrap();
        let empty_string_name = crate::__rt::GDExtensionStringName::from(&empty_string);

        let arguments = [#(#argument_properties),*];
        let argument_metadata = [#(#argument_metadata),*];
        assert_eq!(arguments.len(), argument_metadata.len());

        let return_value_info = #return_value_info;

        let method_info = crate::__rt::GDExtensionClassMethodInfo {
            name: &raw const method_name,
            method_userdata: ::core::ptr::null_mut(),

            call_func: ::core::option::Option::Some(Self::#call_ident),
            ptrcall_func: ::core::option::Option::Some(Self::#ptrcall_ident),
            method_flags: #method_flags,

            has_return_value: return_value_info.is_some(),
            return_value_info: if let ::core::option::Option::Some(info) = return_value_info {
                &raw const info
            } else {
                ::core::ptr::null()
            },
            return_value_metadata: #return_value_metadata,

            argument_count: arguments.len() as u32,
            arguments_info: &raw const arguments as *const *const crate::__rt::GDExtensionPropertyInfo,
            arguments_metadata: &raw const argument_metadata as *const crate::__rt::GDExtensionClassMethodArgumentMetadata,

            default_argument_count: 0,
            default_arguments: std::ptr::null(),
        };

        unsafe { (#api.classdb_register_extension_class_method)(
            #api.library,
            &raw const #class_name_ident,
            &raw const method_info,
        ); }
    }}
}

fn expand(
    impl_block: &ItemImpl,
    attribute_args: &AttributeArgs,
    functions: &[&ImplItemFn],
) -> TokenStream {
    let self_ty = &impl_block.self_ty;

    // Generate a proxy function for each exposed function.
    let mut thunks = Vec::with_capacity(functions.len());
    let mut method_registrations = Vec::with_capacity(functions.len());

    let api_ident = format_ident!("api");
    let class_name_ident = format_ident!("class_name");

    for function in functions {
        let sig = &function.sig;
        let godot_name = sig.ident.to_string();
        let call_ident = format_ident!("__call_{}", sig.ident);
        let ptrcall_ident = format_ident!("__ptrcall_{}", sig.ident);

        let thunk = expand_thunks(&call_ident, &ptrcall_ident, sig);
        thunks.push(thunk);

        let method_registration = expand_method_registration(
            &api_ident,
            &godot_name,
            &class_name_ident,
            sig,
            &call_ident,
            &ptrcall_ident,
        );
        method_registrations.push(method_registration);
    }

    let class_registration = expand_class_registration(&api_ident, self_ty, &class_name_ident);

    let godot_class_name = attribute_args.class_name.to_string();
    quote_spanned! {attribute_args.span=>
        impl #self_ty {
            #(#thunks)*

            fn __pissballs_register(#api_ident: &crate::__rt::GDExtensionAPI) {
                let #class_name_ident = <crate::__rt::GDExtensionString as ::core::str::FromStr>::from_str(#godot_class_name).unwrap();
                let #class_name_ident = crate::__rt::GDExtensionStringName::from(&#class_name_ident);

                #class_registration
                #(#method_registrations)*
            }
        }
    }
}

struct AttributeArgs {
    pub span: Span,
    pub class_name: Ident,
}

impl Parse for AttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();
        let class_name = input.parse::<Ident>()?;
        Ok(Self { span, class_name })
    }
}

/// Attribute macro applied to an `impl` block to expose its methods via GDExtension.
#[proc_macro_error]
#[proc_macro_attribute]
pub fn expose(
    attribute: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // Collect all the functions in the impl block.
    let item = TokenStream::from(item);
    let attribute = TokenStream::from(attribute);

    let impl_block = match syn::parse2::<ItemImpl>(item) {
        Ok(item) => item,
        Err(err) => abort!(err.span(), format!("{err}")),
    };

    let attribute_args = match syn::parse2::<AttributeArgs>(attribute) {
        Ok(item) => item,
        Err(err) => abort!(err.span(), format!("{err}")),
    };

    if let Some((trait_path, _)) = &impl_block.trait_ {
        emit_error!(trait_path, "#[expose] can only be used on plain `impl`s");
    };

    let functions = impl_block
        .items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Fn(f) => Some(f),
            other => {
                emit_error!(other, "#[expose] only supports functions");
                None
            }
        })
        .collect::<Vec<_>>();

    let expanded = expand(&impl_block, &attribute_args, &functions);
    quote! { #impl_block #expanded }.into()
}
