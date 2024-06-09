mod broadcast;
mod disperse;
mod mem_cache;
mod sync_cell;

pub use {broadcast::*, disperse::*, mem_cache::*, sync_cell::*};

#[allow(unused_variables)]
pub trait Storage<K, V>: Clone + Send {
    fn insert_with_default(&mut self, value: V)
    where
        K: Default,
    {
        self.insert(K::default(), value)
    }

    fn insert(&mut self, key: K, value: V) {}

    fn refresh(&mut self) {}

    fn get_item(&self, cursor: usize) -> Option<(&V, usize)>;

    fn get_prev_cursor(&self) -> usize {
        0
    }

    fn size_hint(&self, cursor: usize) -> usize;

    fn with_key(&mut self, key: K) {}
}
