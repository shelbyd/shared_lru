use owning_ref::RwLockReadGuardRef;
use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex, RwLock, Weak},
};

mod allocator;
use allocator::{AllocResult, Allocator, EntryId};
mod lru_map;
mod memory_size;
pub use memory_size::{MemorySize, JustStack};

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

    pub fn make_cache<K, V>(self: &Arc<Self>) -> LruCache<K, V> {
        LruCache {
            shared: Arc::clone(self),
            entry_map: Arc::new(RwLock::new(EntryMap::default())),
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

                unhandled => {
                    unimplemented!("unhandled: {:?}", unhandled);
                }
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
    entry_map: Arc<RwLock<EntryMap<K, V>>>,
}

impl<K, V> LruCache<K, V>
where
    K: MemorySize + Eq + Hash + 'static,
    V: MemorySize + 'static,
{
    pub fn insert(&self, key: K, value: V)
    where
        K: Clone,
    {
        let as_trait: Weak<dyn EntryHolder> =
            Arc::downgrade(&(Arc::clone(&self.entry_map) as Arc<dyn EntryHolder>));

        if let Some(id) = self.shared.claim(key.bytes() + value.bytes(), as_trait) {
            self.entry_map.write().unwrap().insert(id, key, value);
        }
    }

    pub fn get(&self, k: &K) -> Option<RwLockReadGuardRef<EntryMap<K, V>, V>> {
        let read = self.entry_map.read().unwrap();
        match read.get_id(k) {
            Some(id) => {
                self.shared.touch(id);
                Some(RwLockReadGuardRef::new(read).map(|map| map.get(k).unwrap()))
            }
            None => None,
        }
    }
}

trait EntryHolder {
    fn evict(&self, id: EntryId);
}

impl<K, V> EntryHolder for RwLock<EntryMap<K, V>>
where
    K: Eq + Hash,
{
    fn evict(&self, id: EntryId) {
        self.write().unwrap().remove(id);
    }
}

pub struct EntryMap<K, V> {
    values: HashMap<EntryId, V>,
    ids: HashMap<K, EntryId>,
    id_keys: HashMap<EntryId, K>,
}

impl<K, V> EntryMap<K, V>
where
    K: Eq + Hash,
{
    fn insert(&mut self, id: EntryId, key: K, value: V)
    where
        K: Clone,
    {
        self.values.insert(id, value);
        self.ids.insert(key.clone(), id);
        self.id_keys.insert(id, key);
    }

    fn get(&self, key: &K) -> Option<&V> {
        let id = self.ids.get(key)?;
        self.values.get(id)
    }

    fn get_id(&self, key: &K) -> Option<EntryId> {
        self.ids.get(key).cloned()
    }

    fn remove(&mut self, id: EntryId) -> Option<(K, V)> {
        let key = self.id_keys.remove(&id)?;
        self.ids.remove(&key)?;
        let value = self.values.remove(&id)?;
        Some((key, value))
    }
}

impl<K, V> Default for EntryMap<K, V> {
    fn default() -> Self {
        EntryMap {
            values: Default::default(),
            ids: Default::default(),
            id_keys: Default::default(),
        }
    }
}
