use crate::variant::Variant;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Default)]
pub struct Object {
    pub class: String,
    // FIXME: can't use indexmap here because hash/ord but I'd like to preserve order
    pub properties: BTreeMap<String, Variant>,
}

// this shit is SO ass
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum ObjectKind {
    ObjectId(u64),
    Object(Object),
}

impl From<Object> for ObjectKind {
    fn from(value: Object) -> Self {
        Self::Object(value)
    }
}
