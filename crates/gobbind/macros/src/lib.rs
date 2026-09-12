use crate::derive::expand_derive;
use crate::outer_attribute::gobbind_attribute;
use proc_macro2::TokenStream;

mod derive;
mod inner_attribute;
mod outer_attribute;
mod utils;

#[proc_macro_derive(Gobbind, attributes(gobbind))]
pub fn derive_gobbind(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = TokenStream::from(input);
    expand_derive(input).into()
}

#[proc_macro_attribute]
pub fn gobbind(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let attr = TokenStream::from(attr);
    let item = TokenStream::from(item);
    gobbind_attribute(attr, item).into()
}
