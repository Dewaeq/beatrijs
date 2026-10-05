use crate::{defs::Square, gen::ray::line};

pub struct BitBoard;

/// Constant values
/// Ranks and files are in 1-8 notation
impl BitBoard {
    pub const EMPTY: u64 = 0;
    pub const RANK_1: u64 = 0x00000000000000FF;
    pub const RANK_2: u64 = BitBoard::RANK_1 << 8;
    pub const RANK_3: u64 = BitBoard::RANK_1 << 16;
    pub const RANK_4: u64 = BitBoard::RANK_1 << 24;
    pub const RANK_5: u64 = BitBoard::RANK_1 << 32;
    pub const RANK_6: u64 = BitBoard::RANK_1 << 40;
    pub const RANK_7: u64 = BitBoard::RANK_1 << 48;
    pub const RANK_8: u64 = BitBoard::RANK_1 << 56;
    pub const FILE_A: u64 = 0x0101010101010101;
    pub const FILE_B: u64 = BitBoard::FILE_A << 1;
    pub const FILE_C: u64 = BitBoard::FILE_A << 2;
    pub const FILE_D: u64 = BitBoard::FILE_A << 3;
    pub const FILE_E: u64 = BitBoard::FILE_A << 4;
    pub const FILE_F: u64 = BitBoard::FILE_A << 5;
    pub const FILE_G: u64 = BitBoard::FILE_A << 6;
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
        *bb &= *bb - 1;

        lsb
    }

    pub const fn several(bb: u64) -> bool {
        if bb == 0 {
            false
        } else {
            bb & (bb - 1) != 0
        }
    }

    pub const fn only_one(bb: u64) -> bool {
        bb != 0 && (bb & (bb - 1)) == 0
    }

    pub const fn triple_aligned(a: Square, b: Square, c: Square) -> bool {
        line(a, b) & BitBoard::from_sq(c) != 0
    }

    /// Get the index of the least significant bit.
    ///
    /// returns 64 if the provided bitboard is empty.
    pub const fn bit_scan_forward(bb: u64) -> Square {
        bb.trailing_zeros() as _
    }

    /// Get the index of the most significant bit.
    ///
    /// returns 64 if the provided bitboard is empty.
    pub const fn bit_scan_reverse(bb: u64) -> Square {
        if bb == 0 {
            64
        } else {
            63 - bb.leading_zeros() as Square
        }
    }

    pub const fn count(bb: u64) -> u32 {
        bb.count_ones()
    }

    #[allow(dead_code)]
    pub fn pretty_string(bb: u64) -> String {
        let mut output = String::new();
        for y in 0..8 {
            for x in 0..8 {
                let square = 8 * (7 - y) + x;
                let value = (bb >> square) & 1;
                output.push_str(&format!(" {value} "));

                if x == 7 {
                    output.push('\n');
                }
            }
        }
        output
    }
}
