use crate::db::lru::LruCache;
use crate::db::storage::Storage;
use anyhow::{Error, Ok};
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
    cache: Arc<LruCache<K, V>>,
    _marker: PhantomData<(K, V)>,
}

impl<S, K, V> DB<S, K, V>
where
    S: Storage<K, V>,
    K: Hash + Eq + Send + Sync + Clone + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(storage: S, cache_size: usize) -> Self {
        Self {
            storage: Arc::new(storage),
            cache: Arc::new(LruCache::new(cache_size)),
            _marker: PhantomData,
        }
    }

    pub fn get(&self, key: &K) -> Result<Option<Arc<V>>, Error> {
        self.storage.get(key).map_err(Error::from)
    }

    pub fn set(&self, key: K, value: V) -> Result<Option<V>, Error> {
        self.storage.set(key, value).map_err(Error::from)
    }

    pub fn delete(&self, keys: &Vec<K>) -> Result<(), Error> {
        for k in keys.iter() {
            match self.storage.delete(k) {
                Err(e) => {
                    return Err(Error::from(e));
                }
                _ => (),
            }
        }
        Ok(())
    }
}
//EOF
