use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread::yield_now,
};

pub struct VLock {
    pub is_locked: AtomicBool,
}

pub struct VLockGuard {
    lock: *const VLock,
}

impl VLock {
    pub fn new() -> Self {
        Self {
            is_locked: AtomicBool::new(false),
        }
    }

    pub fn try_lock(&self) -> Option<VLockGuard> {
        if !self.is_locked.load(Ordering::Relaxed) {
            if let Ok(_) = self.is_locked.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire) {
                return Some(VLockGuard { lock: self as *const VLock });
            }
        }
        return None;
    }

    pub fn lock(&self) -> VLockGuard {
        loop {
            if let Some(guard) = self.try_lock() {
                return guard;
            }
            yield_now();
        }
    }
}

impl Drop for VLockGuard {
    fn drop(&mut self) {
        unsafe {
            (*self.lock).is_locked.store(false, Ordering::Relaxed);
        }
    }
}
