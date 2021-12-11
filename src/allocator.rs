use core::num::NonZeroUsize;
use lru::LruCache;
use rand::{rngs::SmallRng, Rng, SeedableRng};

pub(crate) struct Allocator {
    used: usize,
    capacity: usize,
    evicting: bool,
    rng: SmallRng,
    allocated: LruCache<EntryId, usize>,
}

impl Allocator {
    pub(crate) fn new(capacity: usize) -> Self {
        Allocator {
            used: 0,
            capacity,
            evicting: false,
            allocated: LruCache::unbounded(),
            rng: SmallRng::from_entropy(),
        }
    }

    pub(crate) fn try_alloc(&mut self, bytes: usize) -> AllocResult {
        if bytes > self.capacity {
            return AllocResult::TooLarge;
        }

        if self.used + bytes > self.capacity {
            if !self.evicting {
                log::info!("Beginning eviction, {}% used", self.percent_used() * 100.);
            }
            self.evicting = true;
        } else if self.used < (self.capacity / 8 * 7) {
            if self.evicting {
                log::info!("Finished evicting, {}% used", self.percent_used() * 100.);
            }
            self.evicting = false;
        }

        if self.evicting {
            let (id, bytes) = self.allocated.pop_lru().expect("should have item");
            self.used -= bytes;
            return AllocResult::Evict(id);
        }

        let id = self.get_id();
        self.allocated.put(id, bytes);
        self.used += bytes;
        AllocResult::Success(id)
    }

    fn get_id(&mut self) -> EntryId {
        loop {
            let id = self.rng.gen::<usize>();
            if let Some(non_zero) = NonZeroUsize::new(id) {
                if !self.allocated.contains(&EntryId(non_zero)) {
                    return EntryId(non_zero);
                }
            }
        }
    }

    pub(crate) fn set_newest(&mut self, id: EntryId) {
        self.allocated.get(&id);
    }

    pub fn percent_used(&self) -> f32 {
        self.used as f32 / self.capacity as f32
    }
}

#[derive(Debug)]
pub(crate) enum AllocResult {
    Success(EntryId),
    Evict(EntryId),
    TooLarge,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntryId(NonZeroUsize);
