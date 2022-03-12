/* use crate::{defs::MAX_GAME_LENGTH, position::Position};

/// Array-based wrapper of game history,
/// so that we can quickly undo moves
#[derive(Clone)]
pub struct History {
    list: [Position; MAX_GAME_LENGTH],
    count: usize,
}

impl History {
    pub fn new() -> Self {
        History {
            list: [Position::new(); MAX_GAME_LENGTH],
            count: 0,
        }
    }

    /// Add a position at `count`
    pub fn push(&mut self, pos: Position) {
        self.list[self.count] = pos;
        self.count += 1;
    }

    /// Decrease `count` by one
    pub fn pop(&mut self) {
        self.count -= 1;
    }

    pub fn current(self) -> Position {
        // *self.list[self.count]
        *self.list.get(0).unwrap()
    }
}
 */