use crate::{
    defs::{Score, Square, NUM_PIECES, NUM_SQUARES},
    params::PIECE_VALUE,
    psqt::PIECE_TABLE,
    utils::mirror,
};

pub const MG_TABLE: [[Score; NUM_SQUARES]; NUM_PIECES * 2] = gen_mg_pesto();
pub const EG_TABLE: [[Score; NUM_SQUARES]; NUM_PIECES * 2] = gen_eg_pesto();

const fn gen_mg_pesto() -> [[Score; NUM_SQUARES]; NUM_PIECES * 2] {
    let mut table = [[0; NUM_SQUARES]; NUM_PIECES * 2];

    let mut piece = 0;
    while piece < NUM_PIECES {
        let mut sq = 0;
        while sq < 64 {
            table[piece][sq] =
                PIECE_VALUE[piece].mg() + PIECE_TABLE[piece][mirror(sq as Square) as usize].mg();
            table[piece + 6][sq] = PIECE_VALUE[piece].mg() + PIECE_TABLE[piece][sq].mg();

            sq += 1;
        }

        piece += 1;
    }

    table
}

const fn gen_eg_pesto() -> [[Score; NUM_SQUARES]; NUM_PIECES * 2] {
    let mut table = [[0; NUM_SQUARES]; NUM_PIECES * 2];

    let mut piece = 0;
    while piece < NUM_PIECES {
        let mut sq = 0;
        while sq < 64 {
            table[piece][sq] =
                PIECE_VALUE[piece].eg() + PIECE_TABLE[piece][mirror(sq as Square) as usize].eg();
            table[piece + 6][sq] = PIECE_VALUE[piece].eg() + PIECE_TABLE[piece][sq].eg();

            sq += 1;
        }

        piece += 1;
    }

    table
}
