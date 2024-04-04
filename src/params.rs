use crate::{
    defs::{Eval, Score, NUM_PIECES},
    e,
};

pub const PIECE_VALUE: [Eval; NUM_PIECES] = [
    e!(126, 208),
    e!(781, 854),
    e!(825, 915),
    e!(1276, 1380),
    e!(2538, 2682),
    e!(0, 0),
];

// Pawn params

/// Passed pawn bonus score, indexed by rank
pub const PASSED_PAWN_SCORE: [Eval; 8] = [
    e!(0, 0),
    e!(-15, 20),
    e!(-25, 35),
    e!(-35, 50),
    e!(5, 65),
    e!(45, 90),
    e!(100, 140),
    e!(0, 0),
];
pub const PAWN_DEFENDED: Eval = e!(5, 9);
pub const PAWN_BEHIND_CENTER: Eval = e!(-20, -4);
pub const BACKWARD_PAWN: Eval = e!(-6, -11);
pub const ISOLATED_PAWN: Eval = e!(-8, -15);
pub const DOUBLED_PAWN: Eval = e!(-11, -19);
pub const PAWN_ATTACK: Eval = e!(7, 0);
pub const PAWN_PUSH: Eval = e!(4, 0);
pub const PAWN_DOUBLE_PUSH: Eval = e!(3, 0);
pub const ROOK_BEHIND_PASSER: Eval = e!(9, 21);
pub const OPP_ROOK_BEHIND_PASSER: Eval = e!(-6, -19);

// Knight params
pub const KNIGHT_PAIR_PENALTY: Eval = e!(-8, -15);
pub const KNIGHT_PAWN_ADJUSTMENT: [Eval; 9] = [
    e!(-20, 8),
    e!(-16, 16),
    e!(-12, 32),
    e!(-8, 35),
    e!(-4, 38),
    e!(2, 33),
    e!(4, 30),
    e!(-11, 27),
    e!(-8, 24),
];
pub const SUPPORTED_KNIGHT: Eval = e!(10, 0);
pub const OUTPOST_KNIGHT: Eval = e!(25, 0);
pub const CONNECTED_KNIGHT: Eval = e!(8, 4);

// Bishop params
pub const BISHOP_PAIR_BONUS: Eval = e!(33, 85);
pub const BISHOP_PAWN_COLOR: Eval = e!(-6, -2);
pub const BISHOP_OPP_PAWN_COLOR: Eval = e!(-8, -3);

// Rook params
pub const ROOK_PAIR_PENALTY: Eval = e!(-8, -22);
pub const ROOK_PAWN_ADJUSTMENT: [Eval; 9] = [
    e!(15, 23),
    e!(-9, 41),
    e!(5, 60),
    e!(5, 45),
    e!(0, 38),
    e!(0, 21),
    e!(-5, 6),
    e!(-25, 6),
    e!(-31, -13),
];
pub const CONNECTED_ROOK: Eval = e!(17, 13);
pub const ROOK_ON_SEVENTH: Eval = e!(11, 16);
pub const ROOK_KING_ALIGNED: Eval = e!(11, 7);

// King params
pub const SHIELD_MISSING: [Eval; 4] = [e!(-2, -1), e!(-23, -14), e!(-38, -21), e!(-55, -29)];
pub const SHIELD_MISSING_ON_OPEN_FILE: [Eval; 4] =
    [e!(-8, -4), e!(-10, -6), e!(-37, -9), e!(-66, -13)];
pub const KING_OPEN: Eval = e!(-13, -4);
pub const KING_SEMI_OPEN: Eval = e!(-5, -1);

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
