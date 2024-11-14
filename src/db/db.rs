use crate::db::storage::Storage;
use anyhow::Error;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;

pub struct DB<S, K, V>
where
    S: Storage<K, V>,
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    storage: Arc<S>,
    _marker: PhantomData<(K, V)>,
}
impl<S, K, V> DB<S, K, V>
where
    S: Storage<K, V>,
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(storage: S) -> Self {
        Self {
            storage: Arc::new(storage),
            _marker: PhantomData,
        }
    }

    pub fn get(&self, key: &K) -> Result<Option<V>, Error> {
        self.storage.get(key).map_err(Error::from)
    }

    pub fn set(&self, key: K, value: V) -> Result<Option<V>, Error> {
        self.storage.set(key, value).map_err(Error::from)
    }

    pub fn delete(&self, key: &K) -> Result<Option<V>, Error> {
        self.storage.delete(key).map_err(Error::from)
    }
}
//EOF
