use crate::{CustomAttributes, FunctionInfo};
use std::any::Any;

pub use linkme;

pub use std::any::TypeId;
use std::mem;

pub type ErasedCustomAttribute = &'static (dyn Any + Send + Sync);
pub type CustomAttributesInner = &'static [ErasedCustomAttribute];

pub const fn new_function_info(
    name: &'static str,
    custom_attributes: CustomAttributesInner,
) -> FunctionInfo {
    let custom_attributes = CustomAttributes::new(custom_attributes);
    FunctionInfo::const_new(name, custom_attributes)
}

#[linkme::distributed_slice]
pub static FUNCTION_INFOS: [(TypeId, FunctionInfo)];

pub const fn type_id_of_val<T: 'static>(t: T) -> TypeId {
    mem::forget(t);
    TypeId::of::<T>()
}
