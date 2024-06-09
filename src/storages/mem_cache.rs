use std::{fmt::Debug, hash::Hash, marker::PhantomData, sync::Arc};

use super::{sync_cell::SyncCell, Storage};

#[derive(Debug)]
pub struct MemCache<K, V>
where
    K: Debug + Clone + Hash + Eq,
{
    cache: Arc<SyncCell<V>>,
    cursor: usize,
    _marker: PhantomData<K>,
}

impl<K, V> Clone for MemCache<K, V>
where
    K: Debug + Clone + Hash + Eq,
{
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            cursor: self.cursor,
            _marker: self._marker,
        }
    }
}

impl<K, V> MemCache<K, V>
where
    K: Debug + Clone + Hash + Eq,
{
    pub fn new(cache: V) -> Self {
        Self {
            cache: Arc::new(SyncCell::new(cache)),
            cursor: 0,
            _marker: PhantomData,
        }
    }
}

impl<K, V> Storage<K, V> for MemCache<K, V>
where
    K: Debug + Clone + Hash + Eq + Send,
{
    fn refresh(&mut self) {
        self.cursor += 1;
    }

    fn get_item(&self, _cursor: usize) -> Option<(&V, usize)> {
        Some((&self.cache.get(), self.cursor))
    }

    fn size_hint(&self, _cursor: usize) -> usize {
        0
    }
}
