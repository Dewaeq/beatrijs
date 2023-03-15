use std::{cell::UnsafeCell, collections::HashSet, mem::MaybeUninit};

const TABLE_SIZE: usize = 100_000;
pub type TT = HashTable<HashEntry, TABLE_SIZE>;

pub trait Table<T, const L: usize>
where
    T: Default + Copy + Entry,
{
    fn new() -> Self;

    fn probe(&self, key: u64, depth: u8) -> Option<T>;

    fn store(&mut self, entry: T);

    fn get(&self, key: u64) -> T;

    fn get_mut(&mut self, key: u64) -> &mut T;
}

pub trait Entry
where
    Self: Default + Copy,
{
    fn key(&self) -> u64;

    fn depth(&self) -> u8;

    fn m(&self) -> u16;

    fn valid(&self) -> bool {
        self.key() != 0
    }
}

pub struct HashTable<T, const L: usize>
where
    T: Default + Copy + Entry,
{
    pub entries: Vec<T>,
    pub size: u64,
}

impl<T, const L: usize> Table<T, L> for HashTable<T, L>
where
    T: Default + Copy + Entry,
{
    fn new() -> Self {
        unsafe {
            let mut entries: Vec<T> = Vec::with_capacity(L);

            for i in 0..L {
                entries.push(T::default());
            }

            HashTable {
                entries,
                size: L as u64,
            }
        }
    }

    fn get(&self, key: u64) -> T {
        unsafe { *self.entries.get_unchecked((key % self.size) as usize) }
    }

    fn get_mut(&mut self, key: u64) -> &mut T {
        unsafe { self.entries.get_unchecked_mut((key % self.size) as usize) }
    }

    fn probe(&self, key: u64, depth: u8) -> Option<T> {
        let entry = self.get(key);

        if entry.valid() && entry.key() == key {
            Some(entry)
        } else {
            None
        }
    }

    fn store(&mut self, entry: T) {
        unsafe {
            let prev = self.get_mut(entry.key());
            if !prev.valid() || !(prev.key() == entry.key() && prev.depth() >= entry.depth()) {
                *prev = entry;
            }
        }
    }
}

impl<T, const L: usize> HashTable<T, L>
where
    T: Default + Copy + Entry,
{
    pub fn best_move(&self, key: u64) -> Option<u16> {
        let entry = self.get(key);
        if entry.valid() && entry.key() == key && entry.m() != 0 {
            Some(entry.m())
        } else {
            None
        }
    }
}

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

impl Entry for HashEntry {
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
}
