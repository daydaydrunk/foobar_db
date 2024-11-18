#![warn(unused_imports)]
use dashmap::DashMap;
use std::borrow::Borrow;
use std::error::Error;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task;

// 自定义错误
#[derive(Debug)]
pub enum StorageError {
    KeyNotFound(String),
    InvalidOperation(String),
    Internal(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyNotFound(key) => write!(f, "Key not found: {}", key),
            Self::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl Error for StorageError {}

pub type Result<T> = std::result::Result<T, StorageError>;

// Storage trait
pub trait Storage<K, V>: Send + Sync + Debug
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn get<Q>(&self, key: &Q) -> Result<Option<Arc<V>>>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq;

    fn set(&self, key: K, value: V) -> Result<Option<V>>;

    fn delete<Q>(&self, key: &Q) -> Result<Option<V>>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq;

    fn clear(&self) -> Result<()>;

    fn len(&self) -> usize;
}

// DashMap Storage implementation
#[derive(Debug)]
pub struct DashMapStorage<K, V>
where
    K: Hash + Eq + Debug,
    V: Debug,
{
    data: DashMap<K, V>,
    state: StorageStats,
}

#[derive(Debug, Default, Clone)]
struct StorageStats {
    operations: u64,
    hits: u64,
    misses: u64,
}

impl<K, V> DashMapStorage<K, V>
where
    K: Hash + Eq + Send + Sync + Debug + 'static,
    V: Clone + Send + Sync + Debug + 'static, // Added Debug trait bound
{
    pub fn new() -> Self {
        Self {
            data: DashMap::new(),
            state: StorageStats {
                operations: 0,
                hits: 0,
                misses: 0,
            },
        }
    }
}

impl<K, V> Storage<K, V> for DashMapStorage<K, V>
where
    K: Hash + Eq + Send + Sync + Clone + Debug + 'static,
    V: Clone + Send + Sync + Debug + 'static, // Added Debug trait bound
{
    fn get<Q>(&self, key: &Q) -> Result<Option<Arc<V>>>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let result = self.data.get(key).map(|r| Arc::new(r.value().clone()));
        Ok(result)
    }

    fn set(&self, key: K, value: V) -> Result<Option<V>> {
        Ok(self.data.insert(key, value))
    }

    fn delete<Q>(&self, key: &Q) -> Result<Option<V>>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        Ok(self.data.remove(key).map(|(_, v)| v))
    }

    fn clear(&self) -> Result<()> {
        self.data.clear();
        Ok(())
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

impl<K, V> Clone for DashMapStorage<K, V>
where
    K: Hash + Eq + Debug + Clone,
    V: Debug + Clone,
{
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            state: self.state.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_operations() {
        let storage: DashMapStorage<String, i32> = DashMapStorage::new();

        // Test set
        assert!(storage.set("key1".to_string(), 100).unwrap().is_none());
        assert!(storage.set("key2".to_string(), 200).unwrap().is_none());

        // Test get
        assert_eq!(*storage.get("key1").unwrap().unwrap(), 100);
        assert_eq!(*storage.get("key2").unwrap().unwrap(), 200);
        assert_eq!(storage.get("nonexistent").unwrap(), None);

        // Test length
        assert_eq!(storage.len(), 2);

        // Test delete
        assert_eq!(storage.delete("key1").unwrap(), Some(100));
        assert_eq!(storage.get("key1").unwrap(), None);
        assert_eq!(storage.len(), 1);

        // Test clear
        storage.clear().unwrap();
        assert_eq!(storage.len(), 0);
        assert_eq!(storage.get("key2").unwrap(), None);
    }

    #[tokio::test]
    async fn test_clone() {
        let storage: DashMapStorage<String, String> = DashMapStorage::new();
        storage
            .set("key1".to_string(), "value1".to_string())
            .unwrap();

        let cloned_storage = storage.clone();
        assert_eq!(
            cloned_storage.get("key1").unwrap().unwrap(),
            Arc::new("value1".to_string())
        );

        // Verify modifications in original don't affect clone
        storage
            .set("key1".to_string(), "modified".to_string())
            .unwrap();
        assert_eq!(
            cloned_storage.get("key1").unwrap().unwrap(),
            Arc::new("value1".to_string())
        );
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let storage = Arc::new(DashMapStorage::<String, i32>::new());
        let storage1 = storage.clone();
        let storage2 = storage.clone();

        let handle1 = tokio::spawn(async move {
            for i in 0..1000 {
                storage1.set(format!("key{}", i), i).unwrap();
            }
        });

        let handle2 = tokio::spawn(async move {
            for i in 1000..2000 {
                storage2.set(format!("key{}", i), i).unwrap();
            }
        });

        // Wait for both tasks to complete
        let _ = tokio::try_join!(handle1, handle2).unwrap();

        assert_eq!(storage.len(), 2000);
    }
}
