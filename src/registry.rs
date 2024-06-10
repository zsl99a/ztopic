use std::{collections::HashMap, hash::Hash, task::Waker};

use futures::task::AtomicWaker;

pub(crate) struct WakerRegistry<K>
where
    K: Hash + Eq,
{
    groups: HashMap<K, HashMap<usize, AtomicWaker>>,
    changed_keys: HashMap<K, bool>,
}

impl<K> Default for WakerRegistry<K>
where
    K: Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K> WakerRegistry<K>
where
    K: Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
            changed_keys: HashMap::new(),
        }
    }

    pub fn new_stream(&mut self, key: K, stream_id: usize) {
        match self.groups.get_mut(&key) {
            Some(group) => group,
            None => self.groups.entry(key).or_default(),
        }
        .insert(stream_id, AtomicWaker::new());
    }

    pub fn drop_stream(&mut self, key: &K, stream_id: usize) {
        self.groups.get_mut(key).and_then(|group| group.remove(&stream_id));
        self.groups.retain(|_, group| !group.is_empty());
        self.changed_keys.retain(|k, _| self.groups.contains_key(k));
    }

    pub fn register(&self, key: &K, stream_id: usize, waker: &Waker) {
        if let Some(w) = self.groups.get(key).and_then(|group| group.get(&stream_id)) {
            w.register(waker)
        }
    }

    pub fn set_changed(&mut self, key: K) {
        if self.groups.contains_key(&key) && !self.changed_keys.contains_key(&key) {
            self.changed_keys.insert(key, true);
        }
    }

    pub fn changed(&self) -> bool {
        !self.changed_keys.is_empty()
    }

    pub fn wake_all(&mut self) {
        for (key, _) in self.changed_keys.drain() {
            if let Some(group) = self.groups.get(&key) {
                for (_, w) in group.iter() {
                    w.wake();
                }
            }
        }
    }
}
