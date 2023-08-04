use crate::defs::{Castling, PieceType, Square, NUM_PIECES, NUM_SIDES};

#[derive(Clone, Debug, Copy)]
pub struct Position {
    /// Castling state.
    ///
    /// Bit 0 is white castle queen side,
    /// bit 1 is white castle king side,
    /// bit 2 is black castle queen side,
    /// bit 3 is black castle king side
    pub castling: u8,
    /// 50 move rule counter
    pub rule_fifty: u8,
    /// Ply at this position, starting from zero
    pub ply: usize,
    /// Square behind the pawn, 64 if none
    pub ep_square: Square,

    /// Zobrist key
    pub key: u64,
    /// Bitboard of all the pieces giving check
    pub checkers_bb: u64,
    /// Per player, bitboard of all the pieces (both colors) blocking check on that player's king
    pub king_blockers: [u64; 2],
    // Per player, bitboard of all the pieces pinning the opponent's king
    pub pinners: [u64; 2],
    /// Per piece type, bitboard containing all the squares on which a piece of that
    /// type gives check to the opponent
    pub check_squares: [u64; NUM_PIECES],
    /// `PIECE_NONE` if none
    pub captured_piece: PieceType,
    pub last_move: Option<u16>,
}

impl Position {
    pub const fn new() -> Self {
        Position {
            castling: Castling::NONE,
            rule_fifty: 0,
            ply: 0,
            key: 0,
            ep_square: 64,
            checkers_bb: 0,
            king_blockers: [0; NUM_SIDES],
            pinners: [0; 2],
            check_squares: [0; NUM_PIECES],
            captured_piece: PieceType::None,
            last_move: None,
        }
    }
}
