use std::{
    cell::SyncUnsafeCell,
};

use crate::{board::Board, search::IS_MATE, defs::Score, movegen::is_legal_move};

pub const TABLE_SIZE_MB: usize = 16;
type TT = HashTable<HashEntry>;

pub trait Table<T> 
where T: Default + Copy,
{
    fn new(num_entries: usize) -> Self;

    fn with_size(mb: usize) -> Self;

    fn clear(&mut self);

    fn probe(&self, key: u64) -> Option<T>;

    fn store(&mut self, entry: T);

    fn get(&self, key: u64) -> T;

    fn get_mut(&mut self, key: u64) -> &mut T;
}

pub struct HashTable<T>
where
    T: Default + Copy,
{
    pub entries: Vec<T>,
    pub size: usize,
}

impl Table<HashEntry> for HashTable<HashEntry> {
    fn new(num_entries: usize) -> Self {
        let entries = vec![HashEntry::default(); num_entries];

        HashTable {
            entries,
            size: num_entries,
        }
    }

    fn with_size(mb: usize) -> Self {
        let num_entries = mb * 1024 * 1024 / std::mem::size_of::<HashEntry>();
        Self::new(num_entries)
    }

    fn clear(&mut self) {
        self.entries = vec![HashEntry::default(); self.size];
    }

    fn probe(&self, key: u64) -> Option<HashEntry> {
        let entry = self.get(key);

        if entry.valid() && entry.key == key {
            Some(entry)
        } else {
            None
        }
    }

    fn store(&mut self, entry: HashEntry) {
        let prev = self.get_mut(entry.key);

        if !prev.valid() 
        // prioritize entries that add a move to a
        // position that previously didnt have a pv move stored
        || (!prev.has_move() && entry.has_move()) 
        || prev.depth < entry.depth {
            *prev = entry;
        }
    }

    fn get(&self, key: u64) -> HashEntry {
        unsafe { *self.entries.get_unchecked(key as usize % self.size) }
    }

    fn get_mut(&mut self, key: u64) -> &mut HashEntry {
        unsafe { self.entries.get_unchecked_mut(key as usize % self.size) }
    }

}

impl HashTable<HashEntry> {
    pub fn best_move(&self, key: u64) -> Option<u16> {
        let entry = self.get(key);
        if entry.valid() && entry.key == key && entry.has_move() {
            Some(entry.m)
        } else {
            None
        }
    }

    pub fn extract_pv(&self, board: &mut Board) -> Vec<u16> {
        let mut pv = vec![];
        let mut m = self.best_move(board.key());

        while let Some(pv_move) = m {
            if pv_move == 0 {
                break;
            }

            if !is_legal_move(board, pv_move) {
                break;
            }

            pv.push(pv_move);
            board.make_move(pv_move);
            m = self.best_move(board.key());
        }

        for _ in 0..pv.len() {
            board.unmake_move(board.pos.last_move.unwrap());
        }

        pv
    }
}

unsafe impl Sync for TWrapper {}
unsafe impl Send for TWrapper {}

pub struct TWrapper {
    pub inner: SyncUnsafeCell<TT>,
}

impl TWrapper {
    pub fn new() -> Self {
        TWrapper {
            inner: SyncUnsafeCell::new(TT::with_size(TABLE_SIZE_MB)),
        }
    }

    pub fn clear(&self) {
        unsafe {
            (*self.inner.get()).clear()
        }
    }

    pub fn probe(&self, key: u64, ply_from_root: u8) -> Option<HashEntry> {
        let mut entry = unsafe {
            (*self.inner.get()).probe(key)
        };

        if let Some(ref mut entry) = entry {
            if entry.score > IS_MATE {
                entry.score -= ply_from_root as Score;
            } else if entry.score < -IS_MATE {
                entry.score += ply_from_root as Score;
            }
        }

        entry
    }

    pub fn store(&self, mut entry: HashEntry, ply_from_root: u8) {
        if entry.score > IS_MATE {
            entry.score += ply_from_root as Score;
        } else if entry.score < -IS_MATE {
            entry.score -= ply_from_root as Score;
        }

        unsafe {
            (*self.inner.get()).store(entry);
        }
    }

    pub fn best_move(&self, key: u64) -> Option<u16> {
        unsafe {
            (*self.inner.get()).best_move(key)
        }
    }

    pub fn extract_pv(&self, board: &mut Board) -> Vec<u16> {
        unsafe {
            (*self.inner.get()).extract_pv(board)
        }
    } 
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NodeType {
    Exact,
    Alpha,
    Beta,
}

#[derive(Copy, Clone, Debug)]
pub struct HashEntry {
    pub key: u64,
    pub depth: u8,
    pub m: u16,
    pub score: Score,
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
    pub fn new(key: u64, depth: u8, m: u16, score: Score, node_type: NodeType) -> Self {
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

    pub const fn has_move(&self) -> bool {
        self.m != 0
    }
}
