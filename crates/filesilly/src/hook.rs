use crate::{Error, os};
use retour::{Function, GenericDetour, HookableWith};
use std::ffi::CStr;
use std::mem;
use std::sync::LazyLock;

pub type LockDetour<T> = LazyLock<SillyHook<T, T>>;

pub struct SillyHook<T: HookableWith<D>, D: Function> {
    hook: crate::Result<GenericDetour<T>>,
    #[allow(dead_code)]
    detour: D,
}

impl<T: HookableWith<D>, D: Function> SillyHook<T, D> {
    fn hook(module: &CStr, export: &CStr, detour: D) -> crate::Result<GenericDetour<T>> {
        unsafe {
            let export = os::get_export(module, export)?;
            let export: T = mem::transmute_copy(&export);
            GenericDetour::new(export, detour).map_err(|_| Error::Hook)
        }
    }

    pub fn new(module: &CStr, export: &CStr, detour: D) -> Self {
        Self {
            hook: SillyHook::hook(module, export, detour),
            detour,
        }
    }

    pub fn get(&self) -> crate::Result<&GenericDetour<T>> {
        match &self.hook {
            Ok(hook) => Ok(hook),
            Err(_) => Err(Error::Hook),
        }
    }

    pub unsafe fn enable(&self) -> crate::Result<()> {
        unsafe { self.get()?.enable().map_err(|_| Error::Hook) }
    }

    pub fn unwrap(&self) -> &GenericDetour<T> {
        self.hook.as_ref().expect("couldn't get hook")
    }
}
