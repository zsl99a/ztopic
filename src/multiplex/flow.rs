use std::{cmp::Ordering, hash::Hash, marker::Send, sync::Arc};

use futures::{stream::BoxStream, Future, StreamExt};
use tokio::{sync::Notify, task::JoinSet};

use crate::{storages::StorageManager, FlowGroup, Storage};

pub struct Flow<K, V, S, F, Fut>
where
    K: Clone + Default + Hash + Eq + Ord,
    S: Storage<V>,
    F: Fn(FlowGroup<K, V, S>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    max_load: usize,
    storage: StorageManager<K, V, S>,
    new_group: F,
    join_sets: Vec<JoinSet<()>>,
    refreshed: Arc<Notify>,
}

impl<K, V, S, F, Fut> Flow<K, V, S, F, Fut>
where
    K: Clone + Default + Hash + Eq + Ord + Send + 'static,
    V: Send + 'static,
    S: Storage<V> + Send + Sync + 'static,
    F: Fn(FlowGroup<K, V, S>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    pub fn new(max_load: usize, storage: StorageManager<K, V, S>, new_group: F) -> Self {
        Self {
            max_load,
            storage,
            new_group,
            join_sets: vec![],
            refreshed: Arc::new(Notify::new()),
        }
    }

    pub fn to_stream(mut self) -> BoxStream<'static, ()> {
        async_stream::stream! {
            loop {
                self.refresh();
                yield;
                self.storage.registry_changed().await;
            }
        }
        .boxed()
    }

    fn refresh(&mut self) {
        let keys = self.storage.registry_keys();
        let group_size = keys.len() / self.max_load + 1;
        loop {
            match self.join_sets.len().cmp(&group_size) {
                Ordering::Less => {
                    let mut join_set = JoinSet::new();
                    join_set.spawn((self.new_group)(FlowGroup::new(
                        self.join_sets.len(),
                        self.max_load,
                        self.refreshed.clone(),
                        self.storage.clone(),
                    )));
                    self.join_sets.push(join_set);
                }
                Ordering::Greater => {
                    self.join_sets.pop();
                }
                Ordering::Equal => break,
            }
        }
        self.refreshed.notify_waiters();
    }
}
