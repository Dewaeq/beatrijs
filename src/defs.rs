use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};

use crate::bitboard::BitBoard;
use crate::params::PIECE_VALUE;

pub const WHITE_IDX: usize = 0;
pub const BLACK_IDX: usize = 1;

pub const MAX_GAME_LENGTH: usize = 512;
pub const MAX_MOVES: usize = 256;
pub const NUM_PIECES: usize = 6;
pub const NUM_SIDES: usize = 2;
pub const NUM_SQUARES: usize = 64;

pub type Square = i8;
pub type Depth = i16;
pub type Score = i32;
pub type TTScore = i16;

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
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Player {
    White = 0,
    Black = 1,
}

impl Player {
    pub const fn opp(&self) -> Self {
        match self {
            Player::White => Player::Black,
            Player::Black => Player::White,
        }
    }

    pub const fn pawn_dir(&self) -> Square {
        match self {
            Player::White => 8,
            Player::Black => -8,
        }
    }

    pub const fn rank_3(&self) -> u64 {
        match self {
            Player::White => BitBoard::RANK_3,
            Player::Black => BitBoard::RANK_6,
        }
    }

    pub const fn rank_7(&self) -> u64 {
        match self {
            Player::White => BitBoard::RANK_7,
            Player::Black => BitBoard::RANK_2,
        }
    }

    pub const fn rank_8(&self) -> u64 {
        match self {
            Player::White => BitBoard::RANK_8,
            Player::Black => BitBoard::RANK_1,
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Piece {
    pub t: PieceType,
    pub c: Player,
}

impl Piece {
    pub const NONE: Piece = Piece::new(PieceType::None, Player::White);

    pub const fn new(piece_type: PieceType, side: Player) -> Self {
        Piece {
            t: piece_type,
            c: side,
        }
    }

    pub fn is_none(&self) -> bool {
        self.t == PieceType::None
    }

    pub fn as_usize(&self) -> usize {
        assert!(self.t != PieceType::None);

        self.t.as_usize() + self.c.as_usize() * 6
    }
}

pub mod pieces {
    use crate::defs::{Piece, PieceType, Player};

    pub const WHITE_PAWN: Piece = Piece::new(PieceType::Pawn, Player::White);
    pub const BLACK_PAWN: Piece = Piece::new(PieceType::Pawn, Player::Black);
    pub const WHITE_KNIGHT: Piece = Piece::new(PieceType::Knight, Player::White);
    pub const BLACK_KNIGHT: Piece = Piece::new(PieceType::Knight, Player::Black);
    pub const WHITE_BISHOP: Piece = Piece::new(PieceType::Bishop, Player::White);
    pub const BLACK_BISHOP: Piece = Piece::new(PieceType::Bishop, Player::Black);
    pub const WHITE_ROOK: Piece = Piece::new(PieceType::Rook, Player::White);
    pub const BLACK_ROOK: Piece = Piece::new(PieceType::Rook, Player::Black);
    pub const WHITE_QUEEN: Piece = Piece::new(PieceType::Queen, Player::White);
    pub const BLACK_QUEEN: Piece = Piece::new(PieceType::Queen, Player::Black);
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

    pub const fn is_none(&self) -> bool {
        matches!(self, PieceType::None)
    }

    pub const fn mg_value(&self) -> Score {
        match self {
            PieceType::None => 0,
            _ => PIECE_VALUE[self.as_usize()].mg(),
        }
    }

    pub const fn eg_value(&self) -> Score {
        match self {
            PieceType::None => 0,
            _ => PIECE_VALUE[self.as_usize()].eg(),
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

#[derive(Clone, Copy, Debug, Default)]
pub struct Eval(i32, i32);

//#[macro_export]
macro_rules! e {
    ($score: expr) => {
        Eval::new($score, $score)
    };

    ($mg: expr, $eg: expr) => {
        Eval::new($mg, $eg)
    };
}

pub(crate) use e;

impl Eval {
    pub const fn new(mg: i32, eg: i32) -> Self {
        Eval(mg, eg)
    }

    pub const fn phased(&self, phase: i32) -> i32 {
        (self.0 * phase + self.1 * (24 - phase)) / 24
    }

    pub const fn mg(&self) -> i32 {
        self.0
    }

    pub const fn eg(&self) -> i32 {
        self.1
    }
}

impl Add for Eval {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        e!(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl AddAssign for Eval {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl Sub for Eval {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        e!(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl SubAssign for Eval {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

impl Mul<i32> for Eval {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        e!(self.0 * rhs, self.1 * rhs)
    }
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
    /// Captures that evade check
    EvadingCaptures,
    /// Captures, non-captures and promotions (everything except evasions)
    /// Only use if not in check
    NonEvasions,
}

pub const SMALL_CENTER: u64 =
    (BitBoard::RANK_4 | BitBoard::RANK_5) & (BitBoard::FILE_D | BitBoard::FILE_E);

pub const DARK_SQUARES: u64 = 0b1010101001010101101010100101010110101010010101011010101001010101;
pub const LIGHT_SQUARES: u64 = !DARK_SQUARES;
