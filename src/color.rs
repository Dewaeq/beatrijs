use crate::{
    bitboard::BitBoard,
    defs::{Player, Square},
};

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

    pub const fn rank_2(&self) -> u64 {
        match self {
            Color::White => BitBoard::RANK_2,
            _ => BitBoard::RANK_7,
        }
    }

    pub const fn rank_3(&self) -> u64 {
        match self {
            Color::White => BitBoard::RANK_3,
            _ => BitBoard::RANK_6,
        }
    }

    pub const fn rank_7(&self) -> u64 {
        match self {
            Color::White => BitBoard::RANK_7,
            _ => BitBoard::RANK_2,
        }
    }

    pub const fn rank_8(&self) -> u64 {
        match self {
            Color::White => BitBoard::RANK_8,
            _ => BitBoard::RANK_1,
        }
    }
}
