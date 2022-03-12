use std::ops::{Index, IndexMut};
use crate::bitboard::BitBoard;

pub const WHITE_IDX: usize = 0;
pub const BLACK_IDX: usize = 1;

pub const MAX_MOVES: usize = 256;
pub const MAX_GAME_LENGTH: usize = 1;
pub const NUM_PIECES: usize = 6;
pub const NUM_SIDES: usize = 2;
pub const NUM_SQUARES: usize = 64;

pub type Square = i8;

pub const DIRS: [i8; 8] = [8, 1, -8, -1, 9, -7, -9, 7];

pub const FEN_START_STRING: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub struct Castling;

impl Castling {
    pub const WQ: u8 = 1;
    pub const WK: u8 = 2;
    pub const BQ: u8 = 4;
    pub const BK: u8 = 8;
    pub const WHITE_ALL: u8 = 3;
    pub const BLACK_ALL: u8 = 12;
    pub const NONE: u8 = 0;
    pub const ALL: u8 = 15;
}

#[derive(Debug, Copy, Clone)]
pub enum Player {
    White,
    Black,
}

impl Player {
    pub const fn opp(&self) -> Self {
        match self {
            &Player::White => Player::Black,
            &Player::Black => Player::White,
        }
    }

    pub const fn pawn_dir(&self) -> Square {
        match self {
            &Player::White => 8,
            &Player::Black => -8,
        }
    }

    pub const fn rank_3(&self) -> u64 {
        match self {
            &Player::White => BitBoard::RANK_3,
            &Player::Black => BitBoard::RANK_6,
        }
    }

    pub const fn rank_7(&self) -> u64 {
        match self {
            &Player::White => BitBoard::RANK_7,
            &Player::Black => BitBoard::RANK_2,
        }
    }

    pub const fn castle_king_sq(&self) -> Square {
        match self {
            Player::White => 6,
            Player::Black => 62,
        }
    }

    pub const fn castle_queen_sq(&self) -> Square {
        match self {
            Player::White => 2,
            Player::Black => 58,
        }
    }

    /// Constant function to use Player as an index in constant contexts
    pub const fn as_usize(self) -> usize {
        self as usize
    }
}

impl Index<Player> for [u64; NUM_SIDES] {
    type Output = u64;

    fn index(&self, index: Player) -> &Self::Output {
        &self[index.as_usize()]
    }
}

impl IndexMut<Player> for [u64; NUM_SIDES] {
    fn index_mut(&mut self, index: Player) -> &mut Self::Output {
        &mut self[index.as_usize()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
    None,
}

impl PieceType {
    /// Constant function to use PieceType as an index in constant contexts
    pub const fn as_usize(self) -> usize {
        match self {
            PieceType::Pawn => 0,
            PieceType::Knight => 1,
            PieceType::Bishop => 2,
            PieceType::Rook => 3,
            PieceType::Queen => 4,
            PieceType::King => 5,
            PieceType::None => 6,
        }
    }
}

impl Index<PieceType> for [u64; NUM_PIECES] {
    type Output = u64;

    fn index(&self, index: PieceType) -> &Self::Output {
        &self[index.as_usize()]
    }
}

impl IndexMut<PieceType> for [u64; NUM_PIECES] {
    fn index_mut(&mut self, index: PieceType) -> &mut Self::Output {
        &mut self[index.as_usize()]
    }
}

#[derive(Clone, Copy)]
pub enum Piece {
    WhitePawn,
    BlackPawn,
    WhiteKnight,
    BlackKnight,
    WhiteBishop,
    BlackBishop,
    WhiteRook,
    BlackRook,
    WhiteQueen,
    BlackQueen,
    WhiteKing,
    BlackKing,
    None,
}

impl Piece {
    pub const fn from_piece_type(piece_type: PieceType, side: Player) -> Self {
        match piece_type {
            PieceType::None => Piece::None,
            PieceType::Pawn => match side {
                Player::White => Piece::WhitePawn,
                _ => Piece::BlackPawn,
            },
            PieceType::Knight => match side {
                Player::White => Piece::WhiteKnight,
                _ => Piece::BlackKnight,
            },
            PieceType::Bishop => match side {
                Player::White => Piece::WhiteBishop,
                _ => Piece::BlackBishop,
            },
            PieceType::Rook => match side {
                Player::White => Piece::WhiteRook,
                _ => Piece::BlackRook,
            },
            PieceType::Queen => match side {
                Player::White => Piece::WhiteQueen,
                _ => Piece::BlackQueen,
            },
            PieceType::King => match side {
                Player::White => Piece::WhiteKing,
                _ => Piece::BlackKing,
            },
        }
    }

    pub const fn to_string(&self) -> &str {
        match self {
            Piece::WhitePawn => "P",
            Piece::BlackPawn => "p",
            Piece::WhiteKnight => "N",
            Piece::BlackKnight => "n",
            Piece::WhiteBishop => "B",
            Piece::BlackBishop => "b",
            Piece::WhiteRook => "R",
            Piece::BlackRook => "r",
            Piece::WhiteQueen => "Q",
            Piece::BlackQueen => "q",
            Piece::WhiteKing => "K",
            Piece::BlackKing => "k",
            Piece::None => " ",
        }
    }
}

/// Rook directions are 0-3
///
/// Bishops directions are 4-7
pub struct Dir;

impl Dir {
    pub const NORTH: usize = 0;
    pub const EAST: usize = 1;
    pub const SOUTH: usize = 2;
    pub const WEST: usize = 3;
    pub const NORTH_EAST: usize = 4;
    pub const SOUTH_EAST: usize = 5;
    pub const SOUTH_WEST: usize = 6;
    pub const NORTH_WEST: usize = 7;

    pub const N_DIRS: usize = 8;
}

#[derive(PartialEq)]
pub enum GenType {
    /// Captures and queen promotions
    Captures,
    /// Non-captures and minor promotions
    Quiets,
    /// Non-captures giving check (castling and promotion not included)
    QuietChecks,
    /// All possible check evasions
    Evasions,
    /// Captures, non-captures and promotions (everything except evasions)
    /// Only use if not in check
    NonEvasions,
}
