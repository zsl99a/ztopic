use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};

use super::{broadcast::Broadcast, sync_cell::SyncCell, Storage};

#[derive(Debug)]
pub struct Disperse<K, V>
where
    K: Debug + Clone + Hash + Eq + Default,
{
    key: K,
    buffers: Arc<SyncCell<HashMap<K, Broadcast<K, V>>>>,
    capacity: usize,
}

impl<K, V> Clone for Disperse<K, V>
where
    K: Debug + Clone + Hash + Eq + Default,
{
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            buffers: self.buffers.clone(),
            capacity: self.capacity,
        }
    }
}

impl<K, V> Disperse<K, V>
where
    K: Debug + Clone + Hash + Eq + Default,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            key: K::default(),
            buffers: Arc::new(SyncCell::new(HashMap::with_capacity(capacity))),
            capacity,
        }
    }
}

impl<K, V> Storage<K, V> for Disperse<K, V>
where
    K: Debug + Clone + Hash + Eq + Default + Send,
{
    fn insert(&mut self, key: K, value: V) {
        match self.buffers.get_mut().get_mut(&key) {
            Some(buffer) => buffer,
            None => self.buffers.get_mut().entry(key.clone()).or_insert(Broadcast::new(self.capacity)),
        }
        .insert(key, value)
    }

    fn get_item(&self, cursor: usize) -> Option<(&V, usize)> {
        self.buffers.get().get(&self.key).and_then(|buffer| buffer.get_item(cursor))
    }

    fn get_prev_cursor(&self) -> usize {
        self.buffers.get().get(&self.key).map(|buffer| buffer.get_prev_cursor()).unwrap_or(0)
    }

    fn size_hint(&self, cursor: usize) -> usize {
        self.buffers.get().get(&self.key).map(|buffer| buffer.size_hint(cursor)).unwrap_or(0)
    }

    fn with_key(&mut self, key: K) {
        self.key = key;
    }
}
