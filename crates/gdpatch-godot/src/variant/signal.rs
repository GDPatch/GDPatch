#[derive(Debug, PartialOrd, Ord, Hash, Eq, PartialEq, Clone)]
pub struct Signal {
    pub name: String,
    pub object_id: u64,
}

impl Signal {
    pub fn new(name: String, object_id: u64) -> Self {
        Self { name, object_id }
    }
}
