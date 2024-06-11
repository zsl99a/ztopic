use std::fmt::Debug;

use crate::Storage;

#[derive(Debug)]
pub struct Broadcast<V> {
    buffer: Vec<Option<V>>,
    capacity: usize,
    cursor: usize,
}

impl<V> Clone for Broadcast<V> {
    fn clone(&self) -> Self {
        Self {
            buffer: (0..self.capacity).map(|_| None).collect(),
            capacity: self.capacity,
            cursor: self.cursor,
        }
    }
}

impl<V> Broadcast<V> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            buffer: (0..capacity).map(|_| None).collect(),
            capacity,
            cursor: 0,
        }
    }

    fn next_cursor(&self, cursor: usize) -> usize {
        let cursor = cursor + 1;
        if cursor < self.capacity {
            cursor
        } else {
            0
        }
    }
}

impl<V> Storage<V> for Broadcast<V> {
    fn insert(&mut self, value: V) {
        let cursor = self.cursor;
        self.buffer[cursor] = Some(value);
        self.cursor = self.next_cursor(cursor);
    }

    fn get_item(&self, cursor: usize) -> Option<(&V, usize)> {
        if cursor == self.cursor {
            return None;
        }
        self.buffer[cursor].as_ref().map(|v| (v, self.next_cursor(cursor)))
    }

    fn get_prev_cursor(&self) -> usize {
        let cursor = self.cursor;
        let prev_cursor = if cursor == 0 { self.capacity - 1 } else { cursor - 1 };
        match self.buffer.get(prev_cursor) {
            Some(_) => prev_cursor,
            None => cursor,
        }
    }

    fn size_hint(&self, cursor: usize) -> usize {
        if cursor < self.cursor {
            self.cursor - cursor
        } else {
            self.capacity - cursor + self.cursor
        }
    }
}
