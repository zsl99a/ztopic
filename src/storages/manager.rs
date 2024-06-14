use std::{collections::BTreeMap, marker::PhantomData, sync::Arc, task::Waker};

use futures::{task::AtomicWaker, Future};
use tokio::sync::Notify;

use crate::{common::SyncCell, Storage};

pub struct StorageManager<K, V, S>
where
    K: Clone + Default + Eq + Ord,
    S: Storage<V>,
{
    stream_key: K,
    inner: Arc<SyncCell<Inner<K, V, S>>>,
}

struct Inner<K, V, S>
where
    K: Clone + Default + Eq + Ord,
    S: Storage<V>,
{
    template: S,
    storages: BTreeMap<K, S>,
    registry: BTreeMap<K, BTreeMap<usize, AtomicWaker>>,
    changed_keys: BTreeMap<K, bool>,
    notify: Notify,
    _marker: PhantomData<V>,
}

impl<K, V, S> Clone for StorageManager<K, V, S>
where
    K: Clone + Default + Eq + Ord,
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
    K: Clone + Default + Eq + Ord,
    S: Storage<V>,
{
    pub fn new(storage: S) -> Self {
        Self {
            stream_key: K::default(),
            inner: Arc::new(SyncCell::new(Inner {
                template: storage.clone(),
                storages: BTreeMap::new(),
                registry: BTreeMap::new(),
                changed_keys: BTreeMap::new(),
                notify: Notify::new(),
                _marker: PhantomData,
            })),
        }
    }

    pub fn insert(&self, value: V) {
        self.insert_with(K::default(), value)
    }

    pub fn insert_with(&self, key: K, value: V) {
        match self.inner_mut().storages.get_mut(&key) {
            Some(buffer) => buffer,
            None => self.inner_mut().storages.entry(key.clone()).or_insert(self.inner.template.clone()),
        }
        .insert(value);
        self.set_changed(key)
    }

    pub fn wake_all(&self) {
        let changed_keys = std::mem::take(&mut self.inner_mut().changed_keys);
        for (key, _) in changed_keys.into_iter() {
            if let Some(wakers) = self.inner.registry.get(&key) {
                for (_, w) in wakers.iter() {
                    w.wake();
                }
            }
        }
    }

    pub fn notified(&self) -> impl Future<Output = ()> + '_ {
        self.inner.notify.notified()
    }
}

impl<K, V, S> StorageManager<K, V, S>
where
    K: Clone + Default + Eq + Ord,
    S: Storage<V>,
{
    pub(crate) fn new_stream(&self, stream_id: usize) {
        match self.inner_mut().registry.get_mut(&self.stream_key) {
            Some(wakers) => wakers,
            None => self.inner_mut().registry.entry(self.stream_key.clone()).or_default(),
        }
        .insert(stream_id, AtomicWaker::new());
        self.inner.notify.notify_waiters();
    }

    pub(crate) fn drop_stream(&self, stream_id: usize) {
        self.inner_mut().registry.get_mut(&self.stream_key).and_then(|wakers| wakers.remove(&stream_id));
        self.inner_mut().registry.retain(|_, wakers| !wakers.is_empty());
        self.inner_mut().changed_keys.retain(|k, _| self.inner.registry.contains_key(k));
        self.inner.notify.notify_waiters();
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

    pub(crate) fn set_changed(&self, key: K) {
        if self.inner.registry.contains_key(&key) && !self.inner.changed_keys.contains_key(&key) {
            self.inner_mut().changed_keys.insert(key, true);
        }
    }
    pub(crate) fn is_changed(&self) -> bool {
        !self.inner.changed_keys.is_empty()
    }

    pub(crate) fn size_hint(&self, cursor: usize) -> usize {
        self.size_hint_with(K::default(), cursor)
    }

    pub(crate) fn size_hint_with(&self, key: K, cursor: usize) -> usize {
        self.inner.storages.get(&key).map(|buffer| buffer.size_hint(cursor)).unwrap_or(0)
    }

    pub(crate) fn registry(&self) -> &BTreeMap<K, BTreeMap<usize, AtomicWaker>> {
        &self.inner.registry
    }

    fn inner_mut(&self) -> &mut Inner<K, V, S> {
        self.inner.get_mut()
    }
}
