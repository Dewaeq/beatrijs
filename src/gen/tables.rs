use crate::{
    bitboard::BitBoard,
    defs::{Score, Square, NUM_SIDES, NUM_SQUARES},
    utils::{b_max, coord_from_square, north_one, south_one},
};

use super::attack::KING_ATK;

/// Manhattan distance:
///
/// The minimal number of orthogonal king moves needed to go from square `a` to square `b`
pub const DISTANCE: [[Score; NUM_SQUARES]; NUM_SQUARES] = gen_distance();

/// Neighbouring files of a given file. If both of them are empty, the pawn is isolated
pub const ISOLATED: [u64; 8] = gen_isolated();

pub const PASSED: [[u64; NUM_SQUARES]; NUM_SIDES] = [gen_white_passed(), gen_black_passed()];

pub const SHIELDING_PAWNS: [[u64; NUM_SQUARES]; NUM_SIDES] =
    [gen_white_shielding(), gen_black_shielding()];

pub const LMR: [[f32; 64]; 32] = gen_lmr();

pub const KING_ZONE: [[u64; NUM_SQUARES]; NUM_SIDES] =
    [gen_white_king_zone(), gen_black_king_zone()];

const fn gen_distance() -> [[Score; NUM_SQUARES]; NUM_SQUARES] {
    let mut table = [[0; NUM_SQUARES]; NUM_SQUARES];

    let mut src = 0;
    while src < 64 {
        let (src_file, src_rank) = coord_from_square(src);

        let mut dest = 0;
        while dest < 64 {
            let (dest_file, dest_rank) = coord_from_square(dest);
            let dist = b_max((dest_rank - src_rank).abs(), (dest_file - src_file).abs());

            table[src as usize][dest as usize] = dist as Score;
            dest += 1;
        }

        src += 1;
    }

    table
}

const fn gen_isolated() -> [u64; 8] {
    let mut table = [0; 8];
    let mut file = 0;
    while file < 8 {
        if file != 0 {
            table[file] |= BitBoard::file_bb((file - 1) as Square);
        }
        if file != 7 {
            table[file] |= BitBoard::file_bb((file + 1) as Square);
        }
        file += 1;
    }

    table
}

const fn gen_white_passed() -> [u64; NUM_SQUARES] {
    let mut table = [0; NUM_SQUARES];
    let mut sq = 0;

    while sq < 64 {
        table[sq] = ISOLATED[sq % 8] | BitBoard::file_bb(sq as Square);
        let mut prev = sq as Square;
        while prev >= 0 {
            table[sq] &= !BitBoard::rank_bb(prev);
            prev -= 8;
        }

        sq += 1;
    }

    table
}

const fn gen_black_passed() -> [u64; NUM_SQUARES] {
    let mut table = [0; NUM_SQUARES];
    let mut sq = 0;

    while sq < 64 {
        table[sq] = ISOLATED[sq % 8] | BitBoard::file_bb(sq as Square);
        let mut prev = sq as Square;
        while prev < 64 {
            table[sq] &= !BitBoard::rank_bb(prev);
            prev += 8;
        }

        sq += 1;
    }

    table
}

const fn gen_white_shielding() -> [u64; NUM_SQUARES] {
    let mut table = [0; NUM_SQUARES];
    let mut sq = 0;

    while sq < 56 {
        let (file, rank) = coord_from_square(sq);
        let mut shield = BitBoard::file_bb(sq);

        if file == 0 {
            shield |= BitBoard::FILE_C;
        } else if file == 7 {
            shield |= BitBoard::FILE_F;
        }

        if file != 0 {
            shield |= BitBoard::file_bb(file - 1);
        }
        if file != 7 {
            shield |= BitBoard::file_bb(file + 1);
        }

        let mut next = sq + 24;
        while next < 64 {
            shield &= !BitBoard::rank_bb(next);
            next += 8;
        }

        let mut prev = sq;
        while prev >= 0 {
            shield &= !BitBoard::rank_bb(prev);
            prev -= 8;
        }

        table[sq as usize] = shield;
        sq += 1;
    }

    table
}

const fn gen_black_shielding() -> [u64; NUM_SQUARES] {
    let mut table = [0; NUM_SQUARES];
    let mut sq = 63;

    while sq > 7 {
        let (file, rank) = coord_from_square(sq);
        let mut shield = BitBoard::file_bb(sq);

        if file == 0 {
            shield |= BitBoard::FILE_C;
        } else if file == 7 {
            shield |= BitBoard::FILE_F;
        }

        if file != 0 {
            shield |= BitBoard::file_bb(file - 1);
        }
        if file != 7 {
            shield |= BitBoard::file_bb(file + 1);
        }

        let mut next = sq - 24;
        while next >= 0 {
            shield &= !BitBoard::rank_bb(next);
            next -= 8;
        }

        let mut prev = sq;
        while prev < 64 {
            shield &= !BitBoard::rank_bb(prev);
            prev += 8;
        }

        table[sq as usize] = shield;

        sq -= 1;
    }

    table
}

include!(concat!(env!("OUT_DIR"), "/logarithms.rs"));
const fn gen_lmr() -> [[f32; 64]; 32] {
    let mut table = [[0f32; 64]; 32];

    let mut depth = 3;
    while depth < 32 {
        let mut move_count = 0;
        while move_count < 64 {
            let d_ln = LN[depth];
            let m_ln = LN[move_count];

            let reduction =
                (0.8422840719846748 * m_ln * d_ln - 0.4 * m_ln - 0.22572624883839026 * d_ln + 1.2);
            if reduction >= 0f32 {
                table[depth][move_count] = reduction;
            }
            move_count += 1;
        }
        depth += 1;
    }

    table
}

const fn gen_white_king_zone() -> [u64; NUM_SQUARES] {
    let mut table = [0; NUM_SQUARES];

    let mut sq = 0;
    while sq < 64 {
        let mut bb = KING_ATK[sq as usize];
        let (_, rank) = coord_from_square(sq);

        // Also include squares in front of king ring
        if rank < 6 {
            bb |= north_one(bb);
        }

        table[sq as usize] = bb & !BitBoard::from_sq(sq);
        sq += 1;
    }

    table
}

const fn gen_black_king_zone() -> [u64; NUM_SQUARES] {
    let mut table = [0; NUM_SQUARES];

    let mut sq = 0;
    while sq < 64 {
        let mut bb = KING_ATK[sq as usize];
        let (_, rank) = coord_from_square(sq);

        // Also include squares in front of king ring
        if rank > 1 {
            bb |= south_one(bb);
        }

        table[sq as usize] = bb & !BitBoard::from_sq(sq);
        sq += 1;
    }

    table
}

#[rustfmt::skip]
/// Center Manhattan distance:
/// 
/// The minimal number of orthogonal king moves, on the otherwise empty board,
/// needed to reach one of the four center squares
pub const CENTER_DISTANCE: [Score; NUM_SQUARES] = [
  6, 5, 4, 3, 3, 4, 5, 6,
  5, 4, 3, 2, 2, 3, 4, 5,
  4, 3, 2, 1, 1, 2, 3, 4,
  3, 2, 1, 0, 0, 1, 2, 3,
  3, 2, 1, 0, 0, 1, 2, 3,
  4, 3, 2, 1, 1, 2, 3, 4,
  5, 4, 3, 2, 2, 3, 4, 5,
  6, 5, 4, 3, 3, 4, 5, 6
];
