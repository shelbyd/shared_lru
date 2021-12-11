use core::num::NonZeroUsize;
use lru::LruCache;
use rand::{rngs::SmallRng, Rng, SeedableRng};

pub(crate) struct Allocator {
    used: usize,
    capacity: usize,
    rng: SmallRng,
    allocated: LruCache<EntryId, usize>,
}

impl Allocator {
    pub(crate) fn new(capacity: usize) -> Self {
        Allocator {
            used: 0,
            capacity,
            allocated: LruCache::unbounded(),
            rng: SmallRng::from_entropy(),
        }
    }

    pub(crate) fn try_alloc(&mut self, bytes: usize) -> AllocResult {
        if self.used + bytes > self.capacity {
            return match self.allocated.pop_lru() {
                Some((id, bytes)) => {
                    self.used -= bytes;
                    AllocResult::Evict(id)
                }
                None => {
                    assert!(bytes > self.capacity);
                    AllocResult::TooLarge
                }
            };
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
}

#[derive(Debug)]
pub(crate) enum AllocResult {
    Success(EntryId),
    Evict(EntryId),
    TooLarge,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntryId(NonZeroUsize);
