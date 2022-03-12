use crate::{board::Board, defs::MAX_MOVES, movegen::generate_legal};

pub struct MoveList {
    moves: [u16; MAX_MOVES],
    count: usize,
    /// Should only be used by iterator implementation
    current: usize,
}

impl MoveList {
    pub const fn new() -> Self {
        MoveList {
            moves: [0; MAX_MOVES],
            count: 0,
            current: 0,
        }
    }

    pub fn legal(board: &mut Board) -> Self {
        let mut move_list = MoveList::new();
        generate_legal(board, &mut move_list);
        move_list
    }

    pub fn push(&mut self, m: u16) {
        self.moves[self.count] = m;
        self.count += 1;
    }

    pub const fn get(&self, index: usize) -> u16 {
        self.moves[index]
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
            Some(self.moves[self.current - 1])
        } else {
            // self.current = 0;
            None
        }
    }
}
