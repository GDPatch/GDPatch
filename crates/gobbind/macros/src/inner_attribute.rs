//! Parsers for `#[gobbind]` attributes within the other macros.

use crate::utils::CombineSynErrors;
use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Error, Expr, LitStr, Token};

pub struct CustomAttribute {
    pub at_token: Token![@],
    pub expr: Expr,
}

impl Parse for CustomAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let at_token = input.parse::<Token![@]>()?;
        let expr = input.parse::<Expr>()?;
        Ok(CustomAttribute { at_token, expr })
    }
}

/// Result of parsing the `#[gobbind]` attributes inside an item.
pub struct InnerAttributes {
    pub rename: Option<LitStr>,
    pub custom_attributes: Vec<CustomAttribute>,
    pub skipped: bool,
}

impl InnerAttributes {
    pub fn parse(attrs: impl Iterator<Item = Attribute>) -> syn::Result<Self> {
        let mut inner_attrs = Vec::new();
        let mut errors = Vec::new();

        for attr in attrs {
            let parsed = attr.parse_args_with(|input: ParseStream| {
                let mut parsed = Vec::new();
                let mut errors = Vec::new();

                loop {
                    if input.is_empty() {
                        break;
                    }

                    match InnerAttribute::parse(input) {
                        Ok(inner) => parsed.push(inner),
                        Err(error) => errors.push(error),
                    };

                    if input.is_empty() {
                        break;
                    }

                    if let Err(error) = input.parse::<Token![,]>() {
                        errors.push(error);
                    }
                }

                if let Some(error) = errors.combine() {
                    Err(error)
                } else {
                    Ok(parsed)
                }
            });

            match parsed {
                Ok(parsed) => inner_attrs.extend(parsed),
                Err(err) => errors.push(err),
            }
        }

        // consolidate inner attributes
        let mut rename = None;
        let mut custom_attributes = Vec::new();
        let mut skipped = false;

        for inner in inner_attrs {
            match inner {
                InnerAttribute::Rename(new_rename) => {
                    if rename.is_some() {
                        let error = Error::new(
                            new_rename.span(),
                            "duplicate #[gobbind(rename = ...)] attributes",
                        );

                        errors.push(error);
                    }

                    rename = Some(new_rename);
                }
                InnerAttribute::Skip => skipped = true,
                InnerAttribute::CustomAttribute(custom_attribute) => {
                    custom_attributes.push(custom_attribute)
                }
            }
        }

        if let Some(error) = errors.combine() {
            Err(error)
        } else {
            Ok(Self {
                rename,
                custom_attributes,
                skipped,
            })
        }
    }
}

enum InnerAttribute {
    /// Renames a method or field to a custom name.
    Rename(LitStr),

    /// Excludes a method or field from code generation.
    Skip,

    /// Adds a custom attribute to the reflected type.
    CustomAttribute(CustomAttribute),
}

impl Parse for InnerAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![@]) {
            let custom_attribute = input.parse::<CustomAttribute>()?;
            return Ok(Self::CustomAttribute(custom_attribute));
        }

        let attr_type = input.parse::<Ident>()?;

        match &*attr_type.to_string() {
            "skip" => Ok(Self::Skip),
            "rename" => {
                input.parse::<Token![=]>()?;
                let rename = input.parse::<LitStr>()?;
                Ok(Self::Rename(rename))
            }

            _ => Err(Error::new(
                attr_type.span(),
                "unknown #[gobbind(...)] attribute",
            )),
        }
    }
}
