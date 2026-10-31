use crate::inner_attribute::{CustomAttribute, InnerAttributes};
use crate::utils::CombineSynErrors;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::parse::Nothing;
use syn::spanned::Spanned;
use syn::{Error, ImplItem, ItemImpl, Safety, Type};

struct Method {
    ident: Ident,
    name: String,
    custom_attributes: Vec<CustomAttribute>,
}

fn parse_items_to_methods(items: &mut [ImplItem]) -> syn::Result<Vec<Method>> {
    let mut methods = Vec::with_capacity(items.len());
    let mut errors = Vec::new();

    for item in items {
        let ImplItem::Fn(item_fn) = item else {
            continue;
        };

        let mut any_errors = false;

        if let Some(asyncness) = &item_fn.sig.asyncness {
            any_errors = true;
            errors.push(Error::new(
                asyncness.span(),
                "`async fn` are currently unsupported",
            ));
        }

        if let Safety::Unsafe(safety) = &item_fn.sig.safety {
            any_errors = true;
            errors.push(Error::new(
                safety.span(),
                "`unsafe fn` are currently unsupported - make a safe wrapper function",
            ));
        }

        if let Some(abi) = &item_fn.sig.abi {
            any_errors = true;
            errors.push(Error::new(
                abi.span(),
                "`extern fn` are currently unsupported - make a Rust ABI wrapper function",
            ));
        }

        // TODO: check for generics

        // remove and parse #[gobbind] attributes
        let attrs = item_fn
            .attrs
            .extract_if(.., |attr| attr.path().is_ident("gobbind"));
        let parsed = match InnerAttributes::parse(attrs) {
            Ok(parsed) => parsed,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };

        if any_errors || parsed.skipped {
            continue;
        }

        let name = match parsed.rename {
            Some(rename) => rename.value(),
            None => item_fn.sig.ident.to_string(),
        };

        methods.push(Method {
            ident: item_fn.sig.ident.clone(),
            name,
            custom_attributes: parsed.custom_attributes,
        });
    }

    if let Some(error) = errors.combine() {
        Err(error)
    } else {
        Ok(methods)
    }
}

fn expand_globals(self_ty: &Type, methods: &[Method]) -> TokenStream {
    let mut function_info_statics = Vec::new();

    for method in methods {
        let name = &method.name;
        let ident = &method.ident;

        let custom_attr_ctors = method.custom_attributes.iter().map(|attr| {
            let expr = &attr.expr;
            quote_spanned! {expr.span()=>
                &const { #expr } as ::gobbind::__private::ErasedCustomAttribute,
            }
        });

        let function_info_ident = format_ident!("__FUNCTION_INFO_{ident}");
        function_info_statics.push(quote! {
            #[::gobbind::__private::linkme::distributed_slice(::gobbind::__private::FUNCTION_INFOS)]
            #[linkme(crate = ::gobbind::__private::linkme)]
            static #function_info_ident: (::gobbind::__private::TypeId, ::gobbind::FunctionInfo) = {
                static __CUSTOM_ATTRIBUTES: ::gobbind::__private::CustomAttributesInner = &[
                    #(#custom_attr_ctors),*
                ];

                let type_id = ::gobbind::__private::type_id_of_val(#self_ty::#ident);
                let info = ::gobbind::__private::new_function_info(#name, __CUSTOM_ATTRIBUTES);
                (type_id, info)
            };
        });
    }

    quote! {
        const _: () = {
            #(#function_info_statics)*
        };
    }
}

pub fn gobbind_attribute(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut errors = Vec::new();

    match syn::parse2::<Nothing>(attr) {
        Ok(_) => {}
        Err(err) => errors.push(err),
    }

    let mut impl_ = {
        let cloned_item = item.clone();

        match syn::parse2::<ItemImpl>(item) {
            Ok(item) => item,
            Err(_) => {
                errors.push(Error::new(
                    cloned_item.span(),
                    "#[gobbind] can only be applied to `impl`s - maybe you forgot the outer attribute?",
                ));
                let error_tokens = errors.combine().unwrap().into_compile_error();

                return quote! {
                    #cloned_item
                    #error_tokens
                };
            }
        }
    };

    if let Some((path, _)) = &impl_.trait_ {
        errors.push(Error::new(
            path.span(),
            "#[gobbind] can only be applied to inherent impls",
        ));
    }

    let self_ty = &*impl_.self_ty;

    let methods = match parse_items_to_methods(&mut impl_.items) {
        Ok(methods) => methods,
        Err(error) => {
            errors.push(error);
            let error = errors.combine().unwrap().into_compile_error();

            return quote! {
                #impl_
                #error
            };
        }
    };

    let globals = expand_globals(self_ty, &methods);

    let tokens = quote! {
        #impl_
        #globals
    };

    // panic!("{}", tokens);
    tokens
}
