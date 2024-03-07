use std::mem;

use imbl::Vector;

pub type SlabKey = u32;

#[derive(Clone, Debug)]
enum SlabEntry<T: Clone> {
    Occupied(T),
    Vacant { next_free: Option<SlabKey> },
}

#[derive(Clone, Debug)]
pub struct Slab<T: Clone> {
    entries: Vector<SlabEntry<T>>,
    next_free: Option<SlabKey>,
    len: usize,
}

impl<T: Clone> Slab<T> {
    pub fn new() -> Self {
        Self {
            entries: Vector::new(),
            next_free: None,
            len: 0,
        }
    }

    pub fn alloc(&mut self, value: T) -> SlabKey {
        assert!(self.len <= u32::MAX as usize);
        if let Some(key) = self.next_free {
            assert!(self.len < self.entries.len());
            let entry = &mut self.entries[key as usize];
            let SlabEntry::Vacant { next_free } = *entry else {
                panic!("Pointer to occupied entry");
            };
            self.next_free = next_free;
            self.len += 1;
            *entry = SlabEntry::Occupied(value);
            key
        } else {
            assert_eq!(self.len, self.entries.len());
            let key = self.entries.len();
            self.entries.push_back(SlabEntry::Occupied(value));
            self.len += 1;
            key as SlabKey
        }
    }

    pub fn free(&mut self, key: SlabKey) -> T {
        let new_entry = SlabEntry::Vacant {
            next_free: self.next_free,
        };
        let entry = mem::replace(&mut self.entries[key as usize], new_entry);
        let SlabEntry::Occupied(value) = entry else {
            panic!("Freed vacant entry");
        };
        self.len -= 1;
        self.next_free = Some(key);
        value
    }

    pub fn get(&self, key: SlabKey) -> Option<&T> {
        if key as usize >= self.entries.len() {
            return None;
        }
        let SlabEntry::Occupied(ref value) = &self.entries[key as usize] else {
            return None;
        };
        Some(value)
    }

    pub fn get_mut(&mut self, key: SlabKey) -> Option<&mut T> {
        if key as usize >= self.entries.len() {
            return None;
        }
        let SlabEntry::Occupied(ref mut value) = &mut self.entries[key as usize] else {
            return None;
        };
        Some(value)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    #[allow(unused)]
    pub fn iter(&self) -> impl Iterator<Item = (SlabKey, &T)> + '_ {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, entry)| match entry {
                SlabEntry::Occupied(ref value) => Some((i as SlabKey, value)),
                SlabEntry::Vacant { .. } => None,
            })
    }
}
