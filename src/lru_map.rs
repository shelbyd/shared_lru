use std::{collections::HashMap, hash::Hash};

#[derive(Debug)]
pub(crate) struct LruMap<K, V> {
    items: HashMap<K, Entry<K, V>>,
    oldest: Option<K>,
    newest: Option<K>,
}

impl<K, V> LruMap<K, V>
where
    K: Eq + Hash + Copy,
{
    pub(crate) fn new() -> Self {
        LruMap {
            items: HashMap::default(),
            oldest: None,
            newest: None,
        }
    }

    pub(crate) fn insert(&mut self, key: K, value: V) {
        let entry = Entry {
            value,
            prev: None,
            next: None,
        };
        self.items.insert(key, entry);

        self.insert_newest(key);
    }

    pub(crate) fn contains_key(&self, key: &K) -> bool {
        self.items.contains_key(key)
    }

    pub(crate) fn take_oldest(&mut self) -> Option<(K, V)> {
        let oldest = self.oldest?;
        self.remove_entry(oldest)?;

        let entry = self.items.remove(&oldest)?;

        Some((oldest, entry.value))
    }

    pub(crate) fn set_newest(&mut self, key: K) -> Option<()> {
        self.remove_entry(key)?;
        self.insert_newest(key)?;
        Some(())
    }

    fn remove_entry(&mut self, key: K) -> Option<()> {
        let entry = self.items.get(&key)?;
        let (prev, next) = (entry.prev, entry.next);

        match prev {
            Some(prev) => self.items.get_mut(&prev)?.next = next,
            None => self.oldest = next,
        }
        match next {
            Some(next) => self.items.get_mut(&next)?.prev = prev,
            None => self.newest = prev,
        }

        Some(())
    }

    fn insert_newest(&mut self, key: K) -> Option<()> {
        let entry = self.items.get_mut(&key)?;
        entry.prev = self.newest;
        entry.next = None;

        match self.oldest {
            Some(_) => {}
            None => self.oldest = Some(key),
        }

        match self.newest {
            None => {}
            Some(newest) => {
                self.items.get_mut(&newest)?.next = Some(key);
            }
        }
        self.newest = Some(key);

        Some(())
    }
}

#[derive(Debug)]
struct Entry<K, V> {
    value: V,
    next: Option<K>,
    prev: Option<K>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pop_is_none() {
        let mut map: LruMap<u8, u8> = LruMap::new();
        assert_eq!(map.take_oldest(), None);
    }

    #[cfg(test)]
    mod set_newest {
        use super::*;

        #[test]
        fn empty() {
            let mut map: LruMap<u8, ()> = LruMap::new();
            assert_eq!(map.set_newest(42), None);
        }

        #[test]
        fn one_item() {
            let mut map: LruMap<u8, ()> = LruMap::new();
            map.insert(42, ());

            assert_eq!(map.set_newest(42), Some(()));
        }

        #[test]
        fn oldest_item() {
            let mut map: LruMap<u8, ()> = LruMap::new();
            map.insert(42, ());
            map.insert(43, ());

            assert_eq!(map.set_newest(42), Some(()));
            assert_eq!(map.take_oldest(), Some((43, ())));
        }

        #[test]
        fn middle_item() {
            let mut map: LruMap<u8, ()> = LruMap::new();
            map.insert(42, ());
            map.insert(43, ());
            map.insert(44, ());

            dbg!(&map);
            map.set_newest(43);
            dbg!(&map);

            assert_eq!(map.take_oldest(), Some((42, ())));
            assert_eq!(map.take_oldest(), Some((44, ())));
            assert_eq!(map.take_oldest(), Some((43, ())));
        }
    }
}
