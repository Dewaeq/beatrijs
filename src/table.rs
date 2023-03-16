use std::{cell::UnsafeCell, collections::HashSet, mem::MaybeUninit};

pub const TABLE_SIZE: usize = 100_000;
pub type TT = HashTable<HashEntry, TABLE_SIZE>;

pub struct HashTable<T, const L: usize>
where
    T: Default + Copy,
{
    pub entries: Vec<T>,
    pub size: u64,
}

impl<const L: usize> HashTable<HashEntry, L>
where
{
    pub fn new() -> Self {
        let mut entries = vec![HashEntry::default(); L];

        HashTable {
            entries,
            size: L as u64,
        }
    }

    pub fn best_move(&self, key: u64) -> Option<u16> {
        let entry = self.get(key);
        if entry.valid() && entry.key() == key && entry.m() != 0 {
            Some(entry.m())
        } else {
            None
        }
    }

    fn get(&self, key: u64) -> HashEntry {
        unsafe { *self.entries.get_unchecked((key % self.size) as usize) }
    }

    fn get_mut(&mut self, key: u64) -> &mut HashEntry {
        unsafe { self.entries.get_unchecked_mut((key % self.size) as usize) }
    }

    pub fn probe(&self, key: u64, depth: u8) -> Option<HashEntry> {
        let entry = self.get(key);

        if entry.valid() && entry.key() == key {
            Some(entry)
        } else {
            None
        }
    }

    pub fn store(&mut self, entry: HashEntry) {
        unsafe {
            let prev = self.get_mut(entry.key());
            if !prev.valid() || !(prev.key() == entry.key() && prev.depth() >= entry.depth()) {
                *prev = entry;
            }
        }
    }
}

impl<T, const L: usize> HashTable<T, L> where T: Default + Copy {}

unsafe impl Sync for TWrapper {}

pub struct TWrapper {
    pub inner: UnsafeCell<TT>,
}

impl TWrapper {
    pub fn new() -> Self {
        TWrapper {
            inner: UnsafeCell::new(TT::new()),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum NodeType {
    Exact,
    Alpha,
    Beta,
}

#[derive(Copy, Clone)]
pub struct HashEntry {
    pub key: u64,
    pub depth: u8,
    pub m: u16,
    pub score: i32,
    pub node_type: NodeType,
}

impl Default for HashEntry {
    fn default() -> Self {
        Self {
            key: 0,
            depth: 0,
            m: 0,
            score: 0,
            node_type: NodeType::Exact,
        }
    }
}

impl HashEntry {
    pub fn new(key: u64, depth: u8, m: u16, score: i32, node_type: NodeType) -> Self {
        HashEntry {
            key,
            depth,
            m,
            score,
            node_type,
        }
    }

    pub const fn valid(&self) -> bool {
        self.key != 0
    }

    fn key(&self) -> u64 {
        self.key
    }

    fn depth(&self) -> u8 {
        self.depth
    }

    fn m(&self) -> u16 {
        self.m
    }
}
