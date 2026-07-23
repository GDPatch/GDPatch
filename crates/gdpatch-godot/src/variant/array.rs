use crate::variant::{ContainerType, Variant};

#[derive(Default, Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Array {
    pub element_type: ContainerType,
    pub inner: Vec<Variant>,
}

impl Array {
    pub fn new(element_type: ContainerType, inner: Vec<Variant>) -> Self {
        Self {
            element_type,
            inner,
        }
    }
}
