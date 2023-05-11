use std::cell::SyncUnsafeCell;

use crate::{board::Board, defs::Score, movegen::is_legal_move, search::IS_MATE};

pub const TABLE_SIZE_MB: usize = 1024;
type TT = HashTable<Bucket>;

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
    pub buckets: Vec<T>,
    pub size: usize,
}

impl Table<Bucket> for HashTable<Bucket> {
    fn new(num_entries: usize) -> Self {
        let size = num_entries / 3;
        let buckets = vec![Bucket::default(); size];

        HashTable {
            buckets,
            size,
        }
    }

    fn with_size(mb: usize) -> Self {
        let num_entries = mb * 1024 * 1024 / std::mem::size_of::<Bucket>() * 3;
        Self::new(num_entries)
    }

    fn clear(&mut self) {
        self.buckets = vec![Bucket::default(); self.size];
    }

    fn probe(&self, key: u64) -> Option<HashEntry> {
        let bucket = self.get(key);
        bucket.probe(key)
    }

    fn store(&mut self, entry: HashEntry) {
        let bucket = self.get_mut(entry.key);
        bucket.store(entry);
    }

    fn get(&self, key: u64) -> Bucket {
        unsafe { *self.buckets.get_unchecked(key as usize % self.size) }
    }

    fn get_mut(&mut self, key: u64) -> &mut Bucket {
        unsafe { self.buckets.get_unchecked_mut(key as usize % self.size) }
    }
}

impl HashTable<HashEntry> {
    pub fn best_move(&self, key: u64) -> Option<u16> {
        let bucket = self.get(key);
        bucket.best_move(key)
        
    }

    pub fn extract_pv(&self, board: &Board, depth: u8) -> Vec<u16> {
        let mut board = board.clone();
        let mut pv = vec![];
        let mut m = self.best_move(board.key());
        let mut i = 0;

        while i < depth && m.is_some() {
            let pv_move = m.unwrap();

            if pv_move == 0 {
                break;
            }

            if !is_legal_move(&mut board, pv_move) {
                break;
            }

            pv.push(pv_move);
            board.make_move(pv_move);
            m = self.best_move(board.key());
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

    pub fn clear(&self) {
        unsafe { (*self.inner.get()).clear() }
    }

    pub fn probe(&self, key: u64, ply_from_root: usize) -> Option<HashEntry> {
        let mut entry = unsafe { (*self.inner.get()).probe(key) };

        if let Some(ref mut entry) = entry {
            if entry.score > IS_MATE {
                entry.score -= ply_from_root as Score;
            } else if entry.score < -IS_MATE {
                entry.score += ply_from_root as Score;
            }
        }

        entry
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
}

#[Derive(Copy, Clone)]
pub struct Bucket {
    pub one: HashEntry,
    pub two: HashEntry,
    pub three: HashEntry,
}

impl Default for Bucket {
    fn default() -> Self {
        Self {
            one: HashEntry::default(),
            two: HashEntry::default(),
            three: HashEntry::default(),
        }
    }
}

impl Bucket {
    pub fn store(&mut self, entry: HashEntry) {
        if self.one.is_better(&entry) {
            self.one = entry;
        } else if self.two.is_better(&entry) {
            self.two = entry;
        } else if self.three.is_better(&entry) {
            self.three = entry;
        }
    }

    pub fn probe(&self, key: u64) -> Option<HashEntry> {
        if self.one.key == key {
            Some(self.one)
        } else if self.two.key == key {
            Some(self.two)
        } else if self.three.key == key {
            Some(self.three)
        }

        None
    }

    pub fn best_move(&self, key: u64) -> Option<u16> {
        if self.one.key == key && self.one.has_move() {
            Some(self.one.m)
        } else if self.two.key == key && self.two.has_move() {
            Some(self.two.m)
        } else if self.three.key == key && self.three.has_move() {
            Some(self.three.m)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HashFlag {
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
    pub static_eval: Score,
    pub hash_flag: HashFlag,
}

impl Default for HashEntry {
    fn default() -> Self {
        Self {
            key: 0,
            depth: 0,
            m: 0,
            score: 0,
            static_eval: 0,
            hash_flag: HashFlag::Exact,
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
        hash_flag: HashFlag,
    ) -> Self {
        HashEntry {
            key,
            depth: depth as u8,
            m,
            score,
            static_eval,
            hash_flag,
        }
    }

    pub const fn valid(&self) -> bool {
        self.key != 0
    }

    pub const fn has_move(&self) -> bool {
        self.m != 0
    }

    /// Is the provided entry better than this one
    /// 
    /// Useful for checking if this entry should be replaced or not
    /// TODO: implement aging
    pub const fn is_better(&self, entry: &HashEntry) -> bool {
        !self.is_valid() || (self.key == entry.key && self.depth < entry.depth)
    }
}
