use std::{fmt::Debug, hash::Hash, marker::PhantomData, sync::Arc};

use super::{sync_cell::SyncCell, Storage};

#[derive(Debug)]
pub struct Broadcast<K, V> {
    inner: Arc<SyncCell<Inner<V>>>,
    _marker: PhantomData<K>,
}

#[derive(Debug)]
struct Inner<V> {
    buffer: Vec<Option<V>>,
    capacity: usize,
    cursor: usize,
}

impl<K, V> Clone for Broadcast<K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _marker: self._marker,
        }
    }
}

impl<K, V> Broadcast<K, V> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            inner: Arc::new(SyncCell::new(Inner {
                buffer: (0..capacity).map(|_| None).collect(),
                capacity,
                cursor: 0,
            })),
            _marker: PhantomData,
        }
    }

    fn next_cursor(&self, cursor: usize) -> usize {
        let cursor = cursor + 1;
        if cursor < self.inner.get().capacity {
            cursor
        } else {
            0
        }
    }
}

impl<K, V> Storage<K, V> for Broadcast<K, V>
where
    K: Debug + Clone + Hash + Eq + Default + Send,
{
    fn insert_with_default(&mut self, value: V) {
        let cursor = self.inner.get().cursor;
        self.inner.get_mut().buffer[cursor] = Some(value);
        self.inner.get_mut().cursor = self.next_cursor(cursor);
    }

    fn get_item(&self, cursor: usize) -> Option<(&V, usize)> {
        if cursor == self.inner.get().cursor {
            return None;
        }
        self.inner.get().buffer[cursor].as_ref().map(|v| (v, self.next_cursor(cursor)))
    }

    fn get_prev_cursor(&self) -> usize {
        let cursor = self.inner.get().cursor;
        let prev_cursor = if cursor == 0 { self.inner.get().capacity - 1 } else { cursor - 1 };
        match self.inner.get().buffer.get(prev_cursor) {
            Some(_) => prev_cursor,
            None => cursor,
        }
    }

    fn size_hint(&self, cursor: usize) -> usize {
        if cursor < self.inner.get().cursor {
            self.inner.get().cursor - cursor
        } else {
            self.inner.get().capacity - cursor + self.inner.get().cursor
        }
    }
}
