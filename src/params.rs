use crate::defs::{Score, NUM_PIECES};

pub const MG_VALUE: [Score; NUM_PIECES] = [126, 781, 825, 1276, 2538, 0];
pub const EG_VALUE: [Score; NUM_PIECES] = [208, 854, 915, 1380, 2682, 0];

/// Passed pawn bonus score, indexed by rank
pub const PASSED_PAWN_SCORE: [Score; 8] = [0, 5, 10, 20, 35, 60, 100, 200];

pub const BISHOP_PAIR_BONUS: Score = 23;
pub const KNIGHT_PAIR_PENALTY: Score = -8;
pub const ROOK_PAIR_PENALTY: Score = -22;
pub const KNIGHT_PAWN_ADJUSTMENT: [Score; 9] = [-20, -16, -12, -8, -4, 0, 4, 8, 12];
pub const ROOK_PAWN_ADJUSTMENT: [Score; 9] = [15, 12, 9, 6, 3, 0, -3, -6, -9];
pub const SUPPORTED_KNIGHT: Score = 10;
pub const OUTPOST_KNIGHT: Score = 25;
pub const CONNECTED_KNIGHT: Score = 8;
pub const CONNECTED_ROOK: Score = 17;
pub const ROOK_ON_SEVENTH: Score = 11;
pub const SHIELD_MISSING: [Score; 4] = [-2, -23, -38, -55];
pub const SHIELD_MISSING_ON_OPEN_FILE: [Score; 4] = [-8, -10, -37, -66];

#[rustfmt::skip]
pub const SAFETY_TABLE: [Score; 100] = [
    0,  0,   1,   2,   3,   5,   7,   9,  12,  15,
    18,  22,  26,  30,  35,  39,  44,  50,  56,  62,
    68,  75,  82,  85,  89,  97, 105, 113, 122, 131,
    140, 150, 169, 180, 191, 202, 213, 225, 237, 248,
    260, 272, 283, 295, 307, 319, 330, 342, 354, 366,
    377, 389, 401, 412, 424, 436, 448, 459, 471, 483,
    494, 500, 500, 500, 500, 500, 500, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500
];
