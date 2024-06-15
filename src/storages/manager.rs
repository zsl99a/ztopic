use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    sync::Arc,
    task::Waker,
};

use futures::{task::AtomicWaker, Future};
use tokio::sync::Notify;

use crate::{common::SyncCell, references::RawRef, Storage};

pub struct StorageManager<K, V, S>
where
    K: Clone + Default + Eq + Hash,
    S: Storage<V>,
{
    stream_key: K,
    inner: Arc<SyncCell<Inner<K, V, S>>>,
}

struct Inner<K, V, S>
where
    K: Clone + Default + Eq + Hash,
    S: Storage<V>,
{
    template: S,
    storages: HashMap<K, S>,
    registry: HashMap<K, HashMap<usize, AtomicWaker>>,
    registry_changed: Notify,
    _marker: PhantomData<V>,
}

impl<K, V, S> Clone for StorageManager<K, V, S>
where
    K: Clone + Default + Eq + Hash,
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
    K: Clone + Default + Eq + Hash,
    S: Storage<V>,
{
    pub fn new(storage: S) -> Self {
        Self {
            stream_key: K::default(),
            inner: Arc::new(SyncCell::new(Inner {
                template: storage.clone(),
                storages: HashMap::new(),
                registry: HashMap::new(),
                registry_changed: Notify::new(),
                _marker: PhantomData,
            })),
        }
    }

    pub fn scope(&self) -> Scope<K, V, S> {
        Scope::new(self)
    }

    pub fn registry_changed(&self) -> impl Future<Output = ()> + '_ {
        self.inner.registry_changed.notified()
    }
}

impl<K, V, S> StorageManager<K, V, S>
where
    K: Clone + Default + Eq + Hash,
    S: Storage<V>,
{
    pub(crate) fn new_stream(&self, stream_id: usize) {
        match self.inner_mut().registry.get_mut(&self.stream_key) {
            Some(wakers) => wakers,
            None => self.inner_mut().registry.entry(self.stream_key.clone()).or_default(),
        }
        .insert(stream_id, AtomicWaker::new());
        self.inner.registry_changed.notify_waiters();
    }

    pub(crate) fn drop_stream(&self, stream_id: usize) {
        let inner = self.inner_mut();
        if let Some(wakers) = inner.registry.get_mut(&self.stream_key) {
            wakers.remove(&stream_id);
            if wakers.is_empty() {
                inner.registry.remove(&self.stream_key);
            }
        }
        self.inner.registry_changed.notify_waiters();
    }

    pub(crate) fn get_prev_cursor(&self) -> usize {
        self.inner.storages.get(&self.stream_key).map(|storage| storage.get_prev_cursor()).unwrap_or(0)
    }

    pub(crate) fn with_key(&mut self, stream_key: K, stream_id: usize) -> usize {
        self.drop_stream(stream_id);
        self.stream_key = stream_key;
        self.new_stream(stream_id);
        self.get_prev_cursor()
    }

    pub(crate) fn get_item(&self, cursor: usize) -> Option<(&V, usize)> {
        self.inner.storages.get(&self.stream_key).and_then(|buffer| buffer.get_item(cursor))
    }

    pub(crate) fn register(&self, stream_id: usize, waker: &Waker) {
        if let Some(w) = self.inner.registry.get(&self.stream_key).and_then(|wakers| wakers.get(&stream_id)) {
            w.register(waker)
        }
    }

    pub(crate) fn size_hint(&self, cursor: usize) -> usize {
        self.size_hint_with(&self.stream_key, cursor)
    }

    pub(crate) fn size_hint_with(&self, key: &K, cursor: usize) -> usize {
        self.inner.storages.get(key).map(|buffer| buffer.size_hint(cursor)).unwrap_or(0)
    }

    pub(crate) fn registry(&self) -> &HashMap<K, HashMap<usize, AtomicWaker>> {
        &self.inner.registry
    }

    fn inner_mut(&self) -> &mut Inner<K, V, S> {
        self.inner.get_mut()
    }
}

pub struct Scope<K, V, S>
where
    K: Clone + Default + Eq + Hash,
    S: Storage<V>,
{
    manager: RawRef<StorageManager<K, V, S>>,
    changed_keys: HashSet<K>,
}

impl<K, V, S> Deref for Scope<K, V, S>
where
    K: Clone + Default + Eq + Hash,
    S: Storage<V>,
{
    type Target = StorageManager<K, V, S>;

    fn deref(&self) -> &Self::Target {
        &self.manager
    }
}

impl<K, V, S> Scope<K, V, S>
where
    K: Clone + Default + Eq + Hash,
    S: Storage<V>,
{
    pub fn new(manager: &StorageManager<K, V, S>) -> Self {
        Self {
            manager: RawRef::from(manager),
            changed_keys: HashSet::new(),
        }
    }

    pub fn insert(&mut self, value: V) {
        self.insert_with(K::default(), value)
    }

    pub fn insert_with(&mut self, key: K, value: V) {
        match self.inner_mut().storages.get_mut(&key) {
            Some(buffer) => buffer,
            None => self.inner_mut().storages.entry(key.clone()).or_insert(self.inner.template.clone()),
        }
        .insert(value);
        self.set_changed(key)
    }

    fn set_changed(&mut self, key: K) {
        self.changed_keys.insert(key);
    }
}

impl<K, V, S> Drop for Scope<K, V, S>
where
    K: Clone + Default + Eq + Hash,
    S: Storage<V>,
{
    fn drop(&mut self) {
        for key in self.changed_keys.iter() {
            if let Some(wakers) = self.inner.registry.get(key) {
                for (_, waker) in wakers.iter() {
                    waker.wake();
                }
            }
        }
    }
}
