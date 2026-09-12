use crate::inner_attribute::{CustomAttribute, InnerAttributes};
use crate::utils::CombineSynErrors;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::parse::Nothing;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Error, FnArg, GenericArgument, ImplItem, ItemImpl, Lifetime, PathArguments, PredicateType,
    ReturnType, Safety, TraitBound, Type, TypeParamBound, WhereClause, WherePredicate, parse_quote,
};

struct Method {
    ident: Ident,
    name: String,
    custom_attributes: Vec<CustomAttribute>,
    argument_types: Vec<Box<Type>>,
    output_type: Box<Type>,
}

/// Replaces all the lifetimes in a type with the provided lifetime.
// Copied from dtolnay/linkme
// https://github.com/dtolnay/linkme/blob/41bcf2f851e811b282d9d1f24bc1b26173488d74impl/src/ty.rs#L3
fn replace_type_lifetime(ty: &mut Type, lt: &Lifetime) {
    match ty {
        Type::Array(ty) => replace_type_lifetime(&mut ty.elem, lt),
        Type::Group(ty) => replace_type_lifetime(&mut ty.elem, lt),
        Type::Paren(ty) => replace_type_lifetime(&mut ty.elem, lt),
        Type::Path(ty) => {
            if let Some(qself) = &mut ty.qself {
                replace_type_lifetime(&mut qself.ty, lt);
            }
            for segment in &mut ty.path.segments {
                if let PathArguments::AngleBracketed(segment) = &mut segment.arguments {
                    for arg in &mut segment.args {
                        if let GenericArgument::Type(arg) = arg {
                            replace_type_lifetime(arg, lt);
                        }
                    }
                }
            }
        }
        Type::Ptr(ty) => replace_type_lifetime(&mut ty.elem, lt),
        Type::Reference(ty) => {
            ty.lifetime = Some(lt.clone());
            replace_type_lifetime(&mut ty.elem, lt);
        }
        Type::Slice(ty) => replace_type_lifetime(&mut ty.elem, lt),
        Type::Tuple(ty) => ty
            .elems
            .iter_mut()
            .for_each(|ty| replace_type_lifetime(ty, lt)),
        Type::ImplTrait(_)
        | Type::Infer(_)
        | Type::Macro(_)
        | Type::Never(_)
        | Type::TraitObject(_)
        | Type::FnPtr(_)
        | Type::Verbatim(_) => {}

        _ => unimplemented!("unknown Type"),
    }
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

        let argument_types = item_fn
            .sig
            .inputs
            .iter()
            .map(|arg| match arg {
                FnArg::Receiver(_) => unimplemented!(),
                FnArg::Typed(pat) => pat.ty.clone(),
            })
            .collect::<Vec<_>>();

        let output_type = match &item_fn.sig.output {
            ReturnType::Default => {
                let ty: Type = syn::parse_quote!(());
                Box::new(ty)
            }
            ReturnType::Type(_, inner) => inner.clone(),
        };

        methods.push(Method {
            ident: item_fn.sig.ident.clone(),
            name,
            custom_attributes: parsed.custom_attributes,
            argument_types,
            output_type,
        });
    }

    if let Some(error) = errors.combine() {
        Err(error)
    } else {
        Ok(methods)
    }
}

fn expand_gobbind_functions_impl(ty: &Type, methods: &[Method]) -> TokenStream {
    let env_lt: Lifetime = parse_quote! { '__env };
    let converter_lt: Lifetime = parse_quote! { '__converter };
    let converter_ty: Type = parse_quote! { __Converter };
    let visitor_ty: Type = parse_quote! { __Visitor };
    let visitor_ident: Ident = parse_quote! { __visitor };

    let where_clause = {
        let mut where_bounds = Vec::new();

        for method in methods {
            for arg_ty in &method.argument_types {
                let mut arg_ty = arg_ty.clone();
                replace_type_lifetime(&mut arg_ty, &converter_lt);

                let bound: TraitBound =
                    parse_quote! { ::gobbind::Provides<#converter_lt, #arg_ty> };
                where_bounds.push(TypeParamBound::Trait(bound));
            }

            let mut output_ty = method.output_type.clone();
            replace_type_lifetime(&mut output_ty, &converter_lt);

            let output_bound: TraitBound =
                parse_quote! { ::gobbind::Consumes<#converter_lt, #output_ty> };
            where_bounds.push(TypeParamBound::Trait(output_bound));
        }

        let predicate = WherePredicate::Type(PredicateType {
            attrs: vec![],
            lifetimes: None,
            bounded_ty: converter_ty.clone(),
            colon_token: Default::default(),
            bounds: Punctuated::from_iter(where_bounds),
        });

        let mut predicates = Punctuated::new();
        predicates.push(predicate);

        WhereClause {
            where_token: Default::default(),
            predicates,
        }
    };

    let mut visit_calls = Vec::new();

    for method in methods {
        let name = &method.name;
        let ident = &method.ident;

        let custom_attr_ctors = method.custom_attributes.iter().map(|attr| {
            let expr = &attr.expr;
            quote_spanned! {expr.span()=>
                &const { #expr } as ::gobbind::__private::ErasedCustomAttribute,
            }
        });

        visit_calls.push(quote! {
            {
                static __CUSTOM_ATTRIBUTES: ::gobbind::__private::CustomAttributesInner = &[
                    #(#custom_attr_ctors),*
                ];

                let info: ::gobbind::FunctionInfo<_, (#converter_ty, _)> = const {
                    ::gobbind::__private::new_function_info(Self::#ident, #name, __CUSTOM_ATTRIBUTES)
                };

                #visitor_ident.visit(info);
            }
        });
    }

    quote! {
        #[automatically_derived]
        impl<#env_lt, #converter_lt, #converter_ty> ::gobbind::GobbindFunctions<#env_lt, #converter_lt, #converter_ty> for #ty
        #where_clause
        {
            fn walk_functions<#visitor_ty>(#visitor_ident: &mut #visitor_ty)
            where
                #visitor_ty: ::gobbind::FunctionVisitor<#env_lt, #converter_lt, Converter = #converter_ty>,
            {
                #(#visit_calls)*
            }
        }
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
            Err(err) => {
                errors.push(err);
                return quote! {
                    #cloned_item
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

    let assert_derives = quote! {
        const _: () = {
            ::gobbind::__private::assert_derives_gobbind::<#self_ty>();
        };
    };

    let methods = match parse_items_to_methods(&mut impl_.items) {
        Ok(methods) => methods,
        Err(error) => {
            errors.push(error);
            let dummy_functions_impl = expand_gobbind_functions_impl(self_ty, &[]);
            let error = errors.combine().unwrap().into_compile_error();

            return quote! {
                #impl_
                #assert_derives
                #dummy_functions_impl

                #error
            };
        }
    };

    let functions_impl = expand_gobbind_functions_impl(self_ty, &methods);

    let tokens = quote! {
        #impl_
        #assert_derives
        #functions_impl
    };

    // panic!("{}", tokens);
    tokens
}
