use crate::variant::{ContainerType, Variant};
use std::collections::BTreeMap;
use std::hash::Hash;

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Dictionary {
    pub key_type: ContainerType,
    pub value_type: ContainerType,
    pub inner: BTreeMap<Variant, Variant>,
}

impl Dictionary {
    pub fn new(
        key_type: ContainerType,
        value_type: ContainerType,
        inner: BTreeMap<Variant, Variant>,
    ) -> Self {
        Self {
            key_type,
            value_type,
            inner,
        }
    }
}
