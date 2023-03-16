use std::{
    cell::{SyncUnsafeCell, UnsafeCell},
    collections::HashSet,
    mem::MaybeUninit,
};

use crate::{board::Board, search::IS_MATE};

pub const TABLE_SIZE: usize = 1_000_000;
pub type TT = HashTable<HashEntry, TABLE_SIZE>;

pub struct HashTable<T, const L: usize>
where
    T: Default + Copy,
{
    pub entries: Vec<T>,
    pub size: u64,
}

impl<const L: usize> HashTable<HashEntry, L> {
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

    pub fn get(&self, key: u64) -> HashEntry {
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

    pub fn store(&mut self, mut entry: HashEntry, ply_from_root: u8) {
        if entry.score > IS_MATE {
            entry.score += ply_from_root as i32;
        } else if entry.score < -IS_MATE {
            entry.score -= ply_from_root as i32;
        }

        let prev = self.get_mut(entry.key());

        if !prev.valid() 
        // prioritize entries that add a move to a
        // position that previously didnt have a pv move stored
        || (prev.m == 0 && entry.m != 0) 
        || prev.depth < entry.depth {
            *prev = entry;
        }

        /* if !prev.valid() || (prev.key() != entry.key()) || (prev.depth() <= entry.depth())
        // || (prev.m == 0 && entry.m != 0)
        {
            *prev = entry;
        } */
    }

    pub fn extract_pv(&self, board: &mut Board) -> Vec<u16> {
        let mut pv = vec![];
        let mut m = self.best_move(board.pos.key);

        while let Some(pv_move) = m {
            if pv_move == 0 {
                break;
            }

            pv.push(pv_move);
            board.make_move(pv_move);
            m = self.best_move(board.pos.key);
        }

        for _ in 0..pv.len() {
            board.unmake_move(board.pos.last_move.unwrap());
        }

        pv
    }
}

impl<T, const L: usize> HashTable<T, L> where T: Default + Copy {}

unsafe impl Sync for TWrapper {}
unsafe impl Send for TWrapper {}

pub struct TWrapper {
    pub inner: SyncUnsafeCell<TT>,
}

impl TWrapper {
    pub fn new() -> Self {
        TWrapper {
            inner: SyncUnsafeCell::new(TT::new()),
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
