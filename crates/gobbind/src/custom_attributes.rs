use std::any::{Any, TypeId};

type Inner = &'static [&'static (dyn Any + Send + Sync)];

/// Container for custom attributes.
#[derive(Debug)]
pub struct CustomAttributes(Inner);

impl CustomAttributes {
    pub(crate) const fn new(inner: Inner) -> Self {
        Self(inner)
    }

    /// Gets a reference to a custom attribute of type `T`.
    pub fn get<T: 'static>(&self) -> Option<&'static T> {
        let wanted_id = TypeId::of::<T>();

        for any in self.0 {
            if (*any).type_id() == wanted_id {
                let downcast = any.downcast_ref::<T>().unwrap();
                return Some(downcast);
            }
        }

        None
    }
}
