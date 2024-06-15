use std::{
    hash::{DefaultHasher, Hash, Hasher},
    ops::Deref,
    sync::Arc,
};

use tokio::sync::Notify;

use crate::{storages::StorageManager, Storage};

pub struct FlowGroup<K, V, S>
where
    K: Clone + Default + Hash + Eq + Ord,
    S: Storage<V>,
{
    index: usize,
    max_load: usize,
    refreshed: Arc<Notify>,
    manager: StorageManager<K, V, S>,
}

impl<K, V, S> Deref for FlowGroup<K, V, S>
where
    K: Clone + Default + Hash + Eq + Ord,
    S: Storage<V>,
{
    type Target = StorageManager<K, V, S>;

    fn deref(&self) -> &Self::Target {
        &self.manager
    }
}

impl<K, V, S> FlowGroup<K, V, S>
where
    K: Clone + Default + Hash + Eq + Ord,
    S: Storage<V>,
{
    pub fn new(index: usize, max_load: usize, refreshed: Arc<Notify>, manager: StorageManager<K, V, S>) -> Self {
        Self {
            index,
            max_load,
            refreshed,
            manager,
        }
    }

    pub async fn refreshed(&self) {
        self.refreshed.notified().await
    }

    pub fn difference(&self, current: &Vec<K>) -> (Vec<K>, Vec<K>) {
        let group_size = self.manager.registry().len() / self.max_load + 1;

        let group_keys = self
            .manager
            .registry()
            .keys()
            .filter_map(|key| {
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                let hash = hasher.finish() as usize;
                if hash % group_size == self.index {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut removing = vec![];
        let mut adding = vec![];

        for c in current {
            if !group_keys.contains(c) {
                removing.push(c.clone());
            }
        }
        for c in &group_keys {
            if !current.contains(c) {
                adding.push(c.clone());
            }
        }

        (removing, adding)
    }
}
