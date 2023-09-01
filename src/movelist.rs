use crate::{
    board::Board,
    defs::MAX_MOVES,
    movegen::{generate_all, generate_legal, generate_quiet},
    search::{HistoryTable, Searcher},
};

#[derive(Clone, Copy)]
pub struct MoveList {
    count: usize,
    /// Should only be used by iterator implementation
    pub current: usize,

    pub entries: [(u16, i32); MAX_MOVES],
}

impl MoveList {
    pub const fn new() -> Self {
        MoveList {
            count: 0,
            current: 0,
            entries: [(0, 0); MAX_MOVES],
        }
    }

    pub fn all(board: &mut Board, history_table: &HistoryTable) -> Self {
        let mut move_list = MoveList::new();
        generate_all(board, history_table, &mut move_list);
        move_list
    }

    pub fn legal(board: &mut Board, history_table: &HistoryTable) -> Self {
        let mut move_list = MoveList::new();
        generate_legal(board, history_table, &mut move_list);
        move_list
    }

    pub fn quiet(board: &mut Board, history_table: &HistoryTable) -> Self {
        let mut move_list = MoveList::new();
        generate_quiet(board, history_table, &mut move_list);
        move_list
    }

    pub fn push(&mut self, m: u16, score: i32) {
        unsafe {
            *self.entries.get_unchecked_mut(self.count) = (m, score);
        }
        self.count += 1;
    }

    pub const fn get_all(&self, index: usize) -> (u16, i32) {
        assert!(index < MAX_MOVES);

        self.entries[index]
    }

    pub const fn get(&self, index: usize) -> u16 {
        assert!(index < MAX_MOVES);

        self.entries[index].0
    }

    pub const fn get_score(&self, index: usize) -> i32 {
        assert!(index < MAX_MOVES);

        self.entries[index].1
    }

    pub fn set_score(&mut self, index: usize, score: i32) {
        unsafe {
            self.entries.get_unchecked_mut(index).1 = score;
        }
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        unsafe {
            let a_ptr: *mut (u16, i32) = &mut self.entries[a];
            let b_ptr: *mut (u16, i32) = &mut self.entries[b];
            std::ptr::swap(a_ptr, b_ptr);
        }
    }

    pub const fn size(&self) -> usize {
        self.count
    }

    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl Iterator for MoveList {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        self.current += 1;

        if self.current <= self.count {
            Some(self.get(self.current - 1))
        } else {
            None
        }
    }
}
