use std::{cmp::Eq, collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData, sync::Arc, task::Waker};

use crate::{common::SyncCell, registry::WakerRegistry};

#[allow(unused_variables)]
pub trait Storage<V>: Clone {
    fn insert(&mut self, value: V) {}

    fn refresh(&mut self) {}

    fn get_item(&self, cursor: usize) -> Option<(&V, usize)>;

    fn get_prev_cursor(&self) -> usize {
        0
    }

    fn size_hint(&self, cursor: usize) -> usize;
}

pub struct StorageManager<K, V, S>
where
    K: Clone + Default + Hash + Eq,
    S: Storage<V>,
{
    stream_key: K,
    inner: Arc<SyncCell<Inner<K, V, S>>>,
}

struct Inner<K, V, S>
where
    K: Clone + Default + Hash + Eq,
    S: Storage<V>,
{
    template: S,
    storages: HashMap<K, S>,
    registry: WakerRegistry<K>,
    _marker: PhantomData<V>,
}

impl<K, V, S> Clone for StorageManager<K, V, S>
where
    K: Clone + Default + Hash + Eq,
    S: Storage<V>,
{
    fn clone(&self) -> Self {
        Self {
            stream_key: self.stream_key.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl<K, V, S> StorageManager<K, V, S>
where
    K: Clone + Default + Hash + Eq + Debug,
    S: Storage<V>,
{
    pub fn new(storage: S) -> Self {
        Self {
            stream_key: K::default(),
            inner: Arc::new(SyncCell::new(Inner {
                template: storage.clone(),
                storages: HashMap::new(),
                registry: WakerRegistry::new(),
                _marker: PhantomData,
            })),
        }
    }

    pub fn new_stream(&self, stream_id: usize) {
        self.registry_mut().new_stream(self.stream_key.clone(), stream_id)
    }

    pub fn drop_stream(&self, stream_id: usize) {
        self.registry_mut().drop_stream(&self.stream_key, stream_id);
    }

    pub fn get_prev_cursor(&self) -> usize {
        self.inner.storages.get(&self.stream_key).map(|storage| storage.get_prev_cursor()).unwrap_or(0)
    }

    pub fn with_key(&mut self, stream_key: K, stream_id: usize) -> usize {
        self.stream_key = stream_key;
        self.drop_stream(stream_id);
        self.new_stream(stream_id);
        self.get_prev_cursor()
    }

    pub fn register(&self, stream_id: usize, waker: &Waker) {
        self.registry_mut().register(&self.stream_key, stream_id, waker)
    }

    pub fn insert(&self, value: V) {
        self.insert_with(K::default(), value)
    }

    pub fn insert_with(&self, key: K, value: V) {
        match self.inner.get_mut().storages.get_mut(&key) {
            Some(buffer) => buffer,
            None => self.inner.get_mut().storages.entry(key.clone()).or_insert(self.inner.template.clone()),
        }
        .insert(value);
        self.inner.get_mut().registry.set_changed(key)
    }

    pub fn size_hint(&self, cursor: usize) -> usize {
        self.size_hint_with(K::default(), cursor)
    }

    pub fn size_hint_with(&self, key: K, cursor: usize) -> usize {
        self.inner.storages.get(&key).map(|buffer| buffer.size_hint(cursor)).unwrap_or(0)
    }

    pub fn get_item(&self, cursor: usize) -> Option<(&V, usize)> {
        self.inner.storages.get(&self.stream_key).and_then(|buffer| buffer.get_item(cursor))
    }

    pub(crate) fn registry(&self) -> &WakerRegistry<K> {
        &self.inner.registry
    }

    pub(crate) fn registry_mut(&self) -> &mut WakerRegistry<K> {
        &mut self.inner.get_mut().registry
    }
}
