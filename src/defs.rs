use crate::bitboard::BitBoard;

pub const WHITE_IDX: usize = 0;
pub const BLACK_IDX: usize = 1;

pub const MAX_MOVES: usize = 256;
pub const NUM_PIECES: usize = 6;
pub const NUM_SIDES: usize = 2;
pub const NUM_SQUARES: usize = 64;

pub const MAX_DEPTH: i32 = 255;

pub type Square = i8;
pub type Score = i32;

pub const INFINITY: Score = Score::MAX - 1;

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

#[derive(Clone, Copy, PartialEq)]
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

pub const PIECES: [Piece; 12] = [
    Piece::new(PieceType::Pawn, Player::White),
    Piece::new(PieceType::Knight, Player::White),
    Piece::new(PieceType::Bishop, Player::White),
    Piece::new(PieceType::Rook, Player::White),
    Piece::new(PieceType::Queen, Player::White),
    Piece::new(PieceType::King, Player::White),
    Piece::new(PieceType::Pawn, Player::Black),
    Piece::new(PieceType::Knight, Player::Black),
    Piece::new(PieceType::Bishop, Player::Black),
    Piece::new(PieceType::Rook, Player::Black),
    Piece::new(PieceType::Queen, Player::Black),
    Piece::new(PieceType::King, Player::Black),
];

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
    /// Captures that evade check
    EvadingCaptures,
    /// Captures, non-captures and promotions (everything except evasions)
    /// Only use if not in check
    NonEvasions,
}

pub const MG_VALUE: [Score; NUM_PIECES] = [126, 781, 825, 1276, 2538, 0];
pub const EG_VALUE: [Score; NUM_PIECES] = [208, 854, 915, 1380, 2682, 0];

/// Passed pawn bonus score, indexed by rank
pub const PASSED_PAWN_SCORE: [Score; 8] = [0, 5, 10, 20, 35, 60, 100, 200];

pub const CASTLE_KING_FILES: u64 = BitBoard::FILE_F | BitBoard::FILE_G | BitBoard::FILE_H;
pub const CASTLE_QUEEN_FILES: u64 = BitBoard::FILE_A | BitBoard::FILE_B | BitBoard::FILE_C;

pub const CENTER_FILES: u64 = BitBoard::FILE_D | BitBoard::FILE_E;
pub const CENTER_SQUARES: u64 = (BitBoard::RANK_4 | BitBoard::RANK_5)
    & (BitBoard::FILE_D | BitBoard::FILE_E);
