use std::{fmt::Debug, ops::Deref};

#[derive(Clone)]
pub struct RawRef<T>(*const T);

impl<T: Debug> Debug for RawRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RawRef").field(&self.0).field(unsafe { &*self.0 }).finish()
    }
}

unsafe impl<T> Send for RawRef<T> {}
unsafe impl<T> Sync for RawRef<T> {}

impl<T> Deref for RawRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl<T> From<&T> for RawRef<T> {
    fn from(value: &T) -> Self {
        Self(value as *const T)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let value = 42;
        let ref_value = RawRef::from(&value);
        assert_eq!(*ref_value, 42);

        println!("{:?}", ref_value);

        println!("{:?}", ref_value.clone())
    }
}
