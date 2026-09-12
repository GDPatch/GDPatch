use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn expand_derive(input: TokenStream) -> TokenStream {
    let input = match syn::parse2::<DeriveInput>(input) {
        Ok(input) => input,
        Err(err) => return err.into_compile_error(),
    };

    let ident = &input.ident;

    quote! {
        #[automatically_derived]
        #[diagnostic::do_not_recommend]
        impl ::gobbind::__private::GobbindAttributeRequiresDeriveOnType for #ident {}
    }
}
