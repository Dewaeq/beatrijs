use crate::defs::{Player, Square};

#[derive(Clone, Copy, PartialEq)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    pub const fn as_usize(self) -> usize {
        self as usize
    }

    pub const fn opp(&self) -> Self {
        match self {
            Color::White => Color::Black,
            _ => Color::White,
        }
    }

    pub const fn pawn_dir(&self) -> Square {
        match self {
            Color::White => 8,
            _ => -8,
        }
    }

    pub const fn to_player(&self) -> Player {
        match self {
            Color::White => Player::White,
            _ => Player::Black,
        }
    }
}
