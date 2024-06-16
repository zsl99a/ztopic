use std::{
    cmp::Ord,
    collections::{BTreeMap, HashSet},
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    sync::Arc,
    task::Waker,
};

use futures::{task::AtomicWaker, Future};
use parking_lot::Mutex;
use tokio::sync::Notify;

use crate::{references::RawRef, Storage};

pub struct StorageManager<K, V, S>
where
    K: Clone + Default + Eq + Hash + Ord,
    S: Storage<V>,
{
    stream_key: K,
    inner: Arc<Mutex<Inner<K, V, S>>>,
}

struct Inner<K, V, S>
where
    K: Clone + Default + Eq + Hash + Ord,
    S: Storage<V>,
{
    template: S,
    storages: BTreeMap<K, S>,
    registry: BTreeMap<K, BTreeMap<usize, AtomicWaker>>,
    registry_changed: Arc<Notify>,
    _marker: PhantomData<V>,
}

impl<K, V, S> Clone for StorageManager<K, V, S>
where
    K: Clone + Default + Eq + Hash + Ord,
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
    K: Clone + Default + Eq + Hash + Ord,
    S: Storage<V>,
{
    pub fn new(storage: S) -> Self {
        Self {
            stream_key: K::default(),
            inner: Arc::new(Mutex::new(Inner {
                template: storage.clone(),
                storages: BTreeMap::new(),
                registry: BTreeMap::new(),
                registry_changed: Arc::new(Notify::new()),
                _marker: PhantomData,
            })),
        }
    }

    pub fn scope(&self) -> Scope<K, V, S> {
        Scope::new(self)
    }

    pub fn registry_changed(&self) -> impl Future<Output = ()> + '_ {
        self.inner().registry_changed.notified()
    }
}

impl<K, V, S> StorageManager<K, V, S>
where
    K: Clone + Default + Eq + Hash + Ord,
    S: Storage<V>,
{
    fn registry_changed_waiters(&self) {
        if Arc::strong_count(&self.inner().registry_changed) == 1 {
            let registry_changed = self.inner().registry_changed.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                registry_changed.notify_waiters();
            });
        }
    }

    pub(crate) fn new_stream(&self, stream_id: usize) {
        let mut inner = self.inner.lock();
        match inner.registry.get_mut(&self.stream_key) {
            Some(wakers) => wakers,
            None => inner.registry.entry(self.stream_key.clone()).or_default(),
        }
        .insert(stream_id, AtomicWaker::new());
        self.registry_changed_waiters();
    }

    pub(crate) fn drop_stream(&self, stream_id: usize) {
        let mut inner = self.inner.lock();
        if let Some(wakers) = inner.registry.get_mut(&self.stream_key) {
            wakers.remove(&stream_id);
            if wakers.is_empty() {
                inner.registry.remove(&self.stream_key);
            }
        }
        self.registry_changed_waiters();
    }

    pub(crate) fn get_prev_cursor(&self) -> usize {
        self.inner()
            .storages
            .get(&self.stream_key)
            .map(|storage| storage.get_prev_cursor())
            .unwrap_or(0)
    }

    pub(crate) fn with_key(&mut self, stream_key: K, stream_id: usize) -> usize {
        self.drop_stream(stream_id);
        self.stream_key = stream_key;
        self.new_stream(stream_id);
        self.get_prev_cursor()
    }

    pub(crate) fn get_item(&self, cursor: usize) -> Option<(&V, usize)> {
        self.inner().storages.get(&self.stream_key).and_then(|buffer| buffer.get_item(cursor))
    }

    pub(crate) fn register(&self, stream_id: usize, waker: &Waker) {
        if let Some(w) = self.inner().registry.get(&self.stream_key).and_then(|wakers| wakers.get(&stream_id)) {
            w.register(waker)
        }
    }

    pub(crate) fn size_hint(&self, cursor: usize) -> usize {
        self.size_hint_with(&self.stream_key, cursor)
    }

    pub(crate) fn size_hint_with(&self, key: &K, cursor: usize) -> usize {
        self.inner().storages.get(key).map(|buffer| buffer.size_hint(cursor)).unwrap_or(0)
    }

    pub(crate) fn registry_keys(&self) -> Vec<K> {
        self.inner.lock().registry.keys().cloned().collect()
    }

    fn inner(&self) -> &Inner<K, V, S> {
        unsafe { &*self.inner.data_ptr() }
    }

    fn inner_mut(&self) -> &mut Inner<K, V, S> {
        unsafe { &mut *self.inner.data_ptr() }
    }
}

pub struct Scope<K, V, S>
where
    K: Clone + Default + Eq + Hash + Ord,
    S: Storage<V>,
{
    manager: RawRef<StorageManager<K, V, S>>,
    changed_keys: HashSet<K>,
}

impl<K, V, S> Deref for Scope<K, V, S>
where
    K: Clone + Default + Eq + Hash + Ord,
    S: Storage<V>,
{
    type Target = StorageManager<K, V, S>;

    fn deref(&self) -> &Self::Target {
        &self.manager
    }
}

impl<K, V, S> Scope<K, V, S>
where
    K: Clone + Default + Eq + Hash + Ord,
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
            None => self.inner_mut().storages.entry(key.clone()).or_insert(self.inner().template.clone()),
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
    K: Clone + Default + Eq + Hash + Ord,
    S: Storage<V>,
{
    fn drop(&mut self) {
        for key in self.changed_keys.iter() {
            if let Some(wakers) = self.inner().registry.get(key) {
                for (_, waker) in wakers.iter() {
                    waker.wake();
                }
            }
        }
    }
}
