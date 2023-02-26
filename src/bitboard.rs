use crate::{defs::Square, gen::ray::line};

#[rustfmt::skip]
const INDEX_64: [Square; 64] = [
    0,  47,  1, 56, 48, 27,  2, 60,
    57, 49, 41, 37, 28, 16,  3, 61,
    54, 58, 35, 52, 50, 42, 21, 44,
    38, 32, 29, 23, 17, 11,  4, 62,
    46, 55, 26, 59, 40, 36, 15, 53,
    34, 51, 20, 43, 31, 22, 10, 45,
    25, 39, 14, 33, 19, 30,  9, 24,
    13, 18,  8, 12,  7,  6,  5, 63,
];

const DEBRUIJN_64: u64 = 0x03f7_9d71_b4cb_0a89;

pub struct BitBoard;

/// Constant values
/// Ranks and files are in 1-8 notation
impl BitBoard {
    pub const EMPTY: u64 = 0;
    pub const RANK_1: u64 = 0x00000000000000FF;
    pub const RANK_2: u64 = BitBoard::RANK_1 << 8;
    pub const RANK_3: u64 = BitBoard::RANK_1 << 16;
    pub const RANK_6: u64 = BitBoard::RANK_1 << 40;
    pub const RANK_7: u64 = BitBoard::RANK_1 << 48;
    pub const FILE_A: u64 = 0x0101010101010101;
    pub const FILE_H: u64 = BitBoard::FILE_A << 7;
}

impl BitBoard {
    pub const fn from_sq(sq: Square) -> u64 {
        1 << sq
    }

    pub const fn file_bb(sq: Square) -> u64 {
        let file = sq % 8;
        BitBoard::FILE_A << file
    }

    pub const fn rank_bb(sq: Square) -> u64 {
        BitBoard::RANK_1 << (sq / 8 * 8)
    }

    pub fn set_bit(bb: &mut u64, sq: Square) {
        *bb |= 1 << sq;
    }

    pub fn pop_bit(bb: &mut u64, sq: Square) {
        *bb ^= 1 << sq;
    }

    pub const fn contains(bb: u64, sq: Square) -> bool {
        BitBoard::from_sq(sq) & bb != 0
    }

    /// Pop the lsb on the provided bitboard and return its index
    ///
    /// Empty bitboards remain empty
    pub fn pop_lsb(bb: &mut u64) -> Square {
        let lsb = BitBoard::bit_scan_forward(*bb);
        if lsb < 64 {
            BitBoard::pop_bit(bb, lsb)
        }
        lsb
    }

    pub const fn more_than_one(bb: u64) -> bool {
        if bb == 0 {
            false
        } else {
            bb & (bb - 1) != 0
        }
    }

    pub const fn triple_aligned(a: Square, b: Square, c: Square) -> bool {
        line(a, b) & BitBoard::from_sq(c) != 0
    }

    /// Get the index of the least significant bit.
    ///
    /// returns 64 if the provided bitboard is empty.
    ///
    /// See <https://www.chessprogramming.org/BitScan#With_separated_LS1B>
    pub const fn bit_scan_forward(bb: u64) -> Square {
        if bb == 0 {
            return 64;
        }

        INDEX_64[(((bb ^ (bb - 1)) * DEBRUIJN_64) >> 58) as usize]
    }

    /// Get the index of the most significant bit.
    ///
    /// returns 64 if the provided bitboard is empty.
    pub const fn bit_scan_reverse(mut bb: u64) -> Square {
        if bb == 0 {
            return 64;
        }

        bb |= bb >> 1;
        bb |= bb >> 2;
        bb |= bb >> 4;
        bb |= bb >> 8;
        bb |= bb >> 16;
        bb |= bb >> 32;

        INDEX_64[((bb * DEBRUIJN_64) >> 58) as usize]
    }

    #[allow(dead_code)]
    pub fn pretty_string(bb: u64) -> String {
        let mut output = String::new();
        for y in 0..8 {
            for x in 0..8 {
                let square = 8 * (7 - y) + x;
                let value = (bb >> square) & 1;
                output.push_str(&format!(" {} ", value));

                if x == 7 {
                    output.push_str("\n");
                }
            }
        }
        output
    }
}
