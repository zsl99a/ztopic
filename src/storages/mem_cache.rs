use std::fmt::Debug;

use super::Storage;

#[derive(Debug, Clone)]
pub struct MemCache<V>
where
    V: Clone,
{
    cache: V,
    cursor: usize,
}

impl<V> MemCache<V>
where
    V: Clone,
{
    pub fn new(cache: V) -> Self {
        Self { cache, cursor: 0 }
    }
}

impl<V> Storage<V> for MemCache<V>
where
    V: Clone,
{
    fn refresh(&mut self) {
        self.cursor += 1;
    }

    fn get_item(&self, _cursor: usize) -> Option<(&V, usize)> {
        Some((&self.cache, self.cursor))
    }

    fn size_hint(&self, _cursor: usize) -> usize {
        0
    }
}
