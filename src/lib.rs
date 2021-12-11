//! An LRU cache that keeps the most recently used values across many different caches.
//!
//! This allows an entire server, for example, to keep K MB of heterogenous memory for cache.
//! Different caches connected to the same SharedLru will use the same "pool" of recency.

use dashmap::{mapref::one::Ref, DashMap};
use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex, Weak},
};

mod allocator;
use allocator::{AllocResult, Allocator, EntryId};
mod memory_size;
pub use memory_size::{JustStack, MemorySize};

pub struct SharedLru {
    inner: Mutex<InnerShared>,
}

impl SharedLru {
    pub fn with_byte_limit(byte_limit: usize) -> Arc<SharedLru> {
        Arc::new(SharedLru {
            inner: Mutex::new(InnerShared {
                allocator: Allocator::new(byte_limit),
                entry_holders: HashMap::new(),
            }),
        })
    }

    pub fn make_cache<K, V>(self: &Arc<Self>) -> LruCache<K, V>
    where
        K: Eq + Hash,
    {
        LruCache {
            shared: Arc::clone(self),
            entry_map: Arc::new(EntryMap::default()),
        }
    }

    fn claim(&self, bytes: usize, holder: Weak<dyn EntryHolder>) -> Option<EntryId> {
        let mut inner = self.inner.lock().unwrap();
        inner.claim(bytes, holder)
    }

    fn touch(&self, id: EntryId) {
        self.inner.lock().unwrap().touch(id)
    }
}

struct InnerShared {
    allocator: Allocator,
    entry_holders: HashMap<EntryId, Weak<dyn EntryHolder>>,
}

impl InnerShared {
    fn claim(&mut self, bytes: usize, holder: Weak<dyn EntryHolder>) -> Option<EntryId> {
        loop {
            match self.allocator.try_alloc(bytes) {
                AllocResult::Success(id) => {
                    self.entry_holders.insert(id, holder);
                    return Some(id);
                }
                AllocResult::Evict(id) => self.evict(id),
                AllocResult::TooLarge => return None,
            }
        }
    }

    fn evict(&mut self, id: EntryId) {
        let holder = self
            .entry_holders
            .remove(&id)
            .expect("should have entry holder for id");
        if let Some(arc) = holder.upgrade() {
            arc.evict(id);
        }
    }

    fn touch(&mut self, id: EntryId) {
        self.allocator.set_newest(id);
    }
}

pub struct LruCache<K, V> {
    shared: Arc<SharedLru>,
    entry_map: Arc<EntryMap<K, V>>,
}

impl<K, V> LruCache<K, V>
where
    K: MemorySize + Eq + Hash + Simple,
    V: MemorySize + Simple,
{
    pub fn insert(&self, key: K, value: V)
    where
        K: Clone,
    {
        // TODO(shelbyd): Remove clone here.
        let as_trait: Weak<dyn EntryHolder> =
            Arc::downgrade(&(Arc::clone(&self.entry_map) as Arc<dyn EntryHolder>));

        if let Some(id) = self.shared.claim(key.bytes() + value.bytes(), as_trait) {
            self.entry_map.insert(id, key, value);
        }
    }

    pub fn get(&self, k: &K) -> Option<ValueRef<K, V>> {
        self.shared.touch(self.entry_map.get_id(k)?);

        Some(ValueRef {
            entry: self.entry_map.get(k)?,
        })
    }

    /// Returns an `Option` because the resulting value may be too large to fit inside the
    /// allowed space. If the value is small enough, this will always return Some.
    pub fn get_or_insert(&self, k: K, insert_with: impl FnOnce() -> V) -> Option<ValueRef<K, V>>
    where
        K: Clone,
    {
        match self.get(&k) {
            Some(ret) => Some(ret),
            None => {
                self.insert(k.clone(), insert_with());
                self.get(&k)
            }
        }
    }
}

pub struct ValueRef<'d, K, V> {
    entry: Ref<'d, EntryId, (K, V)>,
}

impl<'d, K, V> core::ops::Deref for ValueRef<'d, K, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.entry.deref().1
    }
}

pub trait Simple: Send + Sync + 'static {}

impl<T> Simple for T where T: Send + Sync + 'static {}

trait EntryHolder: Simple {
    fn evict(&self, id: EntryId);
}

impl<K, V> EntryHolder for EntryMap<K, V>
where
    K: Eq + Hash + Simple,
    V: Simple,
{
    fn evict(&self, id: EntryId) {
        self.remove(id);
    }
}

pub struct EntryMap<K, V> {
    values: DashMap<EntryId, (K, V)>,
    ids: DashMap<K, EntryId>,
}

impl<K, V> EntryMap<K, V>
where
    K: Eq + Hash,
{
    fn insert(&self, id: EntryId, key: K, value: V)
    where
        K: Clone,
    {
        self.values.insert(id, (key.clone(), value));
        self.ids.insert(key, id);
    }

    fn get(&self, key: &K) -> Option<Ref<EntryId, (K, V)>> {
        let id = self.ids.get(key)?;
        self.values.get(&id)
    }

    fn get_id(&self, key: &K) -> Option<EntryId> {
        self.ids.get(key).map(|id| id.clone())
    }

    fn remove(&self, id: EntryId) -> Option<(K, V)> {
        let (_, (key, value)) = self.values.remove(&id)?;
        self.ids.remove(&key)?;
        Some((key, value))
    }
}

impl<K, V> Default for EntryMap<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        EntryMap {
            values: Default::default(),
            ids: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_sync<T: Sync>() -> bool {
        true
    }
    fn is_send<T: Send>() -> bool {
        true
    }

    #[test]
    fn storage_send_sync() {
        assert!(is_send::<SharedLru>());
        assert!(is_sync::<SharedLru>());
    }

    #[test]
    fn cache_send_sync() {
        assert!(is_send::<LruCache<(), ()>>());
        assert!(is_sync::<LruCache<(), ()>>());
    }
}
