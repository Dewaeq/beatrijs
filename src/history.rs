use std::slice::Iter;

use crate::{defs::MAX_GAME_LENGTH, position::Position};

#[derive(Copy, Clone)]
pub struct History {
    positions: [Position; MAX_GAME_LENGTH],
    pub count: usize,
}

impl History {
    pub const fn new() -> Self {
        History {
            positions: [Position::new(); MAX_GAME_LENGTH],
            count: 0,
        }
    }

    pub fn clear(&mut self) {
        self.count = 0;
    }

    pub fn push(&mut self, pos: Position) {
        assert!(self.count < MAX_GAME_LENGTH);

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
        self.positions[index].key
    }

    pub fn iter(&self) -> Iter<'_, Position> {
        self.positions.iter()
    }
}
