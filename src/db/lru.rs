use std::collections::HashMap;
use std::collections::VecDeque;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};

// LRU Cache entry
struct Entry<V> {
    value: Arc<V>,
    expiry: Option<Instant>,
}
// LRU Cache implementation
pub struct LruCache<K, V> {
    map: HashMap<K, (V, Instant)>,
    queue: VecDeque<K>,
    capacity: usize,
}

impl<K, V> LruCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            queue: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some((value, _)) = self.map.get(key) {
            // Move to front of queue
            if let Some(index) = self.queue.iter().position(|x| x == key) {
                self.queue.remove(index);
                self.queue.push_front(key.clone());
            }
            Some(value.clone())
        } else {
            None
        }
    }

    pub fn put(&mut self, key: K, value: V) {
        // Remove oldest if at capacity
        if self.map.len() >= self.capacity {
            if let Some(old_key) = self.queue.pop_back() {
                self.map.remove(&old_key);
            }
        }

        // Insert new entry
        self.map.insert(key.clone(), (value, Instant::now()));
        self.queue.push_front(key);
    }
}
