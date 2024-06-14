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
