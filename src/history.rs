/* use crate::position::Position;

/// Array-based wrapper of game history,
/// so that we can quickly undo moves
pub struct History {
    list: Vec<Position>,
    count: usize,
}

impl History {
    pub fn new() -> Self {
        History {
            list: vec![],
            count: 0,
        }
    }

    /// Add a position at `count`
    pub fn push(&mut self, pos: Position) {
        self.list.push(pos);
        self.count += 1;
    }

    /// Decrease `count` by one
    pub fn pop(&mut self) {
        self.list.pop();
        self.count -= 1;
    }

    pub fn current(&self) -> Position {
        self.list[self.count - 1].clone()
    }
}
 */