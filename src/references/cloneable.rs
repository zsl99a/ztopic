use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct Cloneable<T>(T);

impl<T> Deref for Cloneable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<&T> for Cloneable<T>
where
    T: Clone,
{
    fn from(value: &T) -> Self {
        Self(value.clone())
    }
}
