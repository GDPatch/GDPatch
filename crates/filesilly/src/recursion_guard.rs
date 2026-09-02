use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct RecursionGuard(AtomicBool);

impl RecursionGuard {
    pub const fn new() -> Self {
        Self(AtomicBool::new(false))
    }

    pub fn acquire(&self) -> Option<HeldRecursionGuard<'_>> {
        let acquired = self.0.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok();

        if acquired {
            Some(HeldRecursionGuard(self))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct HeldRecursionGuard<'a>(&'a RecursionGuard);

impl Drop for HeldRecursionGuard<'_> {
    fn drop(&mut self) {
        self.0.0.store(false, Ordering::Release);
    }
}