use crate::{
    board::Board,
    defs::MAX_MOVES,
    movegen::{generate_all, generate_legal, generate_quiet, MovegenParams},
    search::{HistoryTable, Searcher},
};

//#[derive(Clone, Copy)]
pub struct MoveList {
    moves: [u16; MAX_MOVES],
    scores: [i32; MAX_MOVES],
    count: usize,
    /// Should only be used by iterator implementation
    current: usize,
}

impl MoveList {
    pub const fn new() -> Self {
        MoveList {
            moves: [0; MAX_MOVES],
            scores: [0; MAX_MOVES],
            count: 0,
            current: 0,
        }
    }

    pub fn all(params: MovegenParams) -> Self {
        let mut move_list = MoveList::new();
        generate_all(&params, &mut move_list);
        move_list
    }

    pub fn legal(params: MovegenParams) -> Self {
        let mut move_list = MoveList::new();
        generate_legal(&params, &mut move_list);
        move_list
    }

    pub fn quiet(params: MovegenParams) -> Self {
        let mut move_list = MoveList::new();
        generate_quiet(&params, &mut move_list);
        move_list
    }

    pub fn push(&mut self, m: u16, score: i32) {
        unsafe {
            *self.moves.get_unchecked_mut(self.count) = m;
            *self.scores.get_unchecked_mut(self.count) = score;
        }
        self.count += 1;
    }

    pub const fn get_all(&self, index: usize) -> (u16, i32) {
        assert!(index < MAX_MOVES);
        (self.moves[index], self.scores[index])
    }

    pub const fn get(&self, index: usize) -> u16 {
        assert!(index < MAX_MOVES);
        self.moves[index]
    }

    pub const fn get_score(&self, index: usize) -> i32 {
        assert!(index < MAX_MOVES);
        self.scores[index]
    }

    pub fn set_score(&mut self, index: usize, score: i32) {
        unsafe {
            *self.scores.get_unchecked_mut(index) = score;
        }
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        unsafe {
            let a_ptr: *mut u16 = &mut self.moves[a];
            let b_ptr: *mut u16 = &mut self.moves[b];
            std::ptr::swap(a_ptr, b_ptr);

            let a_score_ptr: *mut i32 = &mut self.scores[a];
            let b_score_ptr: *mut i32 = &mut self.scores[b];
            std::ptr::swap(a_score_ptr, b_score_ptr)
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
