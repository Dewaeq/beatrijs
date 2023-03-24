use crate::{board::Board, defs::MAX_MOVES, position::Position};

#[derive(Copy, Clone)]
pub struct History {
    positions: [Position; MAX_MOVES],
    pub count: usize,
}

impl History {
    pub const fn new() -> Self {
        History {
            positions: [Position::new(); MAX_MOVES],
            count: 0,
        }
    }

    pub fn push(&mut self, pos: Position) {
        unsafe {
            *self.positions.get_unchecked_mut(self.count) = pos;
        }
        self.count += 1;
    }

    pub fn pop(&mut self) -> Position {
        assert!(self.count >= 1);

        self.count -= 1;
        unsafe { *self.positions.get_unchecked(self.count) }
    }

    pub const fn empty(&self) -> bool {
        self.count == 0
    }

    pub const fn get_key(&self, index: usize) -> u64 {
        unsafe { self.positions.get_unchecked(index).key }
    }
}
