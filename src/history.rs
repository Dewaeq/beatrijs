use crate::{defs::MAX_MOVES, position::Position, board::Board};

pub struct History {
    positions: [Position; MAX_MOVES],
    count: usize,
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
        unsafe {
            *self.positions.get_unchecked(self.count)
        }
    }
}
