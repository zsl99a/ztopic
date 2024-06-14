use std::{fmt::Debug, ops::Deref, ptr::NonNull};

#[derive(Clone)]
pub struct NonNullRef<T>(NonNull<T>);

impl<T: Debug> Debug for NonNullRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NonNullRef").field(&self.0).field(unsafe { &*self.0.as_ptr() }).finish()
    }
}

unsafe impl<T> Send for NonNullRef<T> {}
unsafe impl<T> Sync for NonNullRef<T> {}

impl<T> Deref for NonNullRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.as_ptr() }
    }
}

impl<T> From<&T> for NonNullRef<T> {
    fn from(value: &T) -> Self {
        Self(NonNull::from(value))
    }
}
