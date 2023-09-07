use crate::{
    defs::Score,
    search::{INFINITY, IS_MATE},
    speed::{board::Board, movegen::MoveGen},
};
use std::cell::SyncUnsafeCell;

pub const TABLE_SIZE_MB: usize = 128;
type TT = HashTable<HashEntry>;

pub trait Table<T>
where
    T: Default + Copy,
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
        *prev = entry;

        // TODO: add aging to table entries,
        // the method below is very inefficient, especially in endgames
        /* if !prev.valid()
        // prioritize entries that add a move to a
        // position that previously didnt have a pv move stored
        || (!prev.has_move() && entry.has_move())
        || prev.depth < entry.depth {
            *prev = entry;
        } */
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

    pub fn extract_pv(&self, board: &Board, depth: u8) -> Vec<u16> {
        let mut board = board.clone();
        let mut pv = vec![];
        let mut m = self.best_move(board.hash());
        let mut i = 0;

        while i < depth && m.is_some() {
            let pv_move = m.unwrap();

            if pv_move == 0 {
                break;
            }

            let mut legals = MoveGen::simple(&board);
            if !legals.any(|m| m == pv_move) {
                break;
            }

            pv.push(pv_move);
            board = board.make_move(pv_move);
            m = self.best_move(board.hash());
            i += 1;
        }

        pv
    }

    pub fn hash_full(&self) -> usize {
        let mut filled = 0;
        let mut total = 0;

        let mut index = 0;
        while index < self.size && filled < 500 {
            if self.entries[index].valid() {
                filled += 1;
            }
            total += 1;
            index += 1;
        }

        index = self.size - 1;
        while filled < 1000 && index > 0 {
            if self.entries[index].valid() {
                filled += 1;
            }
            total += 1;
            index -= 1;
        }

        (filled as f64 / total as f64 * 1000f64) as usize
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

    pub fn with_size(mb: usize) -> Self {
        TWrapper {
            inner: SyncUnsafeCell::new(TT::with_size(mb)),
        }
    }

    pub fn clear(&self) {
        unsafe { (*self.inner.get()).clear() }
    }

    pub fn probe(&self, key: u64, ply_from_root: usize) -> (bool, HashEntry) {
        let mut entry = unsafe { (*self.inner.get()).get(key) };

        if entry.key == key {
            if entry.score > IS_MATE {
                entry.score -= ply_from_root as Score;
            } else if entry.score < -IS_MATE {
                entry.score += ply_from_root as Score;
            }

            return (true, entry);
        }

        (false, entry)
    }

    pub fn store(&self, mut entry: HashEntry, ply_from_root: usize) {
        if entry.score > IS_MATE {
            entry.score += ply_from_root as Score;
        } else if entry.score < -IS_MATE {
            entry.score -= ply_from_root as Score;
        }

        unsafe {
            (*self.inner.get()).store(entry);
        }
    }

    pub fn store_eval(&self, key: u64, eval: Score) {
        unsafe {
            *(*self.inner.get()).get_mut(key) =
                HashEntry::new(key, 0, 0, -INFINITY, eval, Bound::None);
        }
    }

    pub fn delete(&self, key: u64) {
        unsafe {
            *(*self.inner.get()).get_mut(key) = HashEntry::default();
        }
    }

    pub fn best_move(&self, key: u64) -> Option<u16> {
        unsafe { (*self.inner.get()).best_move(key) }
    }

    pub fn extract_pv(&self, board: &mut Board, depth: i32) -> Vec<u16> {
        unsafe { (*self.inner.get()).extract_pv(board, depth as u8) }
    }

    pub fn hash_full(&self) -> usize {
        unsafe { (*self.inner.get()).hash_full() }
    }

    pub fn size_mb(&self) -> usize {
        unsafe { (*self.inner.get()).size * std::mem::size_of::<HashEntry>() / (1024 * 1024) }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Bound {
    Exact,
    Upper,
    Lower,
    None,
}

#[derive(Copy, Clone, Debug)]
pub struct HashEntry {
    pub key: u64,
    pub depth: u8,
    pub m: u16,
    pub score: Score,
    pub static_eval: Score,
    pub bound: Bound,
}

impl Default for HashEntry {
    fn default() -> Self {
        Self {
            key: 0,
            depth: 0,
            m: 0,
            score: 0,
            static_eval: 0,
            bound: Bound::Exact,
        }
    }
}

impl HashEntry {
    pub fn new(
        key: u64,
        depth: i32,
        m: u16,
        score: Score,
        static_eval: Score,
        hash_flag: Bound,
    ) -> Self {
        HashEntry {
            key,
            depth: depth as u8,
            m,
            score,
            static_eval,
            bound: hash_flag,
        }
    }

    pub const fn valid(&self) -> bool {
        self.key != 0
    }

    pub const fn has_move(&self) -> bool {
        self.m != 0
    }
}
