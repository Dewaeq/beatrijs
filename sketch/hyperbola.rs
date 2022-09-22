/* use crate::{bitboard::BitBoard, defs::Square, utils::coord_from_square};

pub fn test() {}

fn attack(sq: Square, occ: u64, mask: u64) -> u64 {
    let blockers = occ & mask;

    ((blockers - (1 << sq)) ^ bit_swap(bit_swap(blockers) - (1 << (sq ^ 56)))) & mask
}

/* fn horizontal_attack(sq: Square, occ: u64) -> u64 {
    let file = (sq & 7) as u64;
    let rank_x8 = sq & 56; // rank * 8
    let rank_occ_x2 = (occ >> rank_x8) & 126;
    let attacks = RANK_ATTACKS[(4 * rank_occ_x2 + file) as usize];

    attacks << rank_x8
} */

fn bit_swap(mut bb: u64) -> u64 {
    bb = ((bb >> 8) & 0x00FF00FF00FF00FF) | ((bb << 8) & 0xFF00FF00FF00FF00);
    bb = ((bb >> 16) & 0x0000FFFF0000FFFF) | ((bb << 16) & 0xFFFF0000FFFF0000);
    bb = ((bb >> 32) & 0x00000000FFFFFFFF) | ((bb << 32) & 0xFFFFFFFF00000000);

    bb
}

/* fn straight_moves(sq: Square, occ: u64) -> u64 {
    let sq_bb = 1 << sq;
    let file = coord_from_square(sq).0 as usize;
    let rank = coord_from_square(sq).1 as usize;

    let file_mask = FileMasks8[file];
    let rank_mask = RankMasks8[rank];

    let hor = (occ - 2 * sq_bb) ^ bit_swap(bit_swap(occ) - 2 * bit_swap(sq_bb));
    let vert = ((occ & file_mask) - (2 * sq_bb))
        ^ bit_swap(bit_swap(occ & file_mask) - (2 * bit_swap(sq_bb)));

    BitBoard::print_bitboard(hor);
    BitBoard::print_bitboard(vert);

    (hor & rank_mask) | (vert & file_mask)
}

fn diagonal_moves(sq: Square, occ: u64) -> u64 {
    let sq_bb = 1 << sq;
    let (file, rank) = coord_from_square(sq);
    let sum = (file + rank) as usize;
    let sub_sum = (rank + 7 - file) as usize;

    let diag_mask = DiagonalMasks8[sum];
    let anti_mask = AntiDiagonalMasks8[sub_sum];

    let diag = ((occ & diag_mask) - (2 * sq_bb))
        ^ bit_swap(bit_swap(occ & diag_mask) - (2 * bit_swap(sq_bb)));
    let anti = ((occ & anti_mask) - (2 * sq_bb))
        ^ bit_swap(bit_swap(occ & anti_mask) - (2 * bit_swap(sq_bb)));

    (diag & diag_mask) | (anti & anti_mask)
}

pub fn bit_swap(mut bb: u64) -> u64 {
    bb = (bb & 0x5555555555555555) << 1 | ((bb >> 1) & 0x5555555555555555);
    bb = (bb & 0x3333333333333333) << 2 | ((bb >> 2) & 0x3333333333333333);
    bb = (bb & 0x0f0f0f0f0f0f0f0f) << 4 | ((bb >> 4) & 0x0f0f0f0f0f0f0f0f);
    bb = (bb & 0x00ff00ff00ff00ff) << 8 | ((bb >> 8) & 0x00ff00ff00ff00ff);

    (bb << 48) | ((bb & 0xffff0000) << 16) | ((bb >> 16) & 0xffff0000) | (bb >> 48)
} */

const fn get_rank_attacks() -> [u64; 512] {
    let mut rank_attack = [0; 512];

    let mut sq: Square = 0;
    while sq < 64 {
        let mut f = 0;
        while f < 8 {
            let o = (2 * sq) as u64;
            let mut y2: u64 = 0;

            let mut x2 = f as Square - 1;
            loop {
                let b = 1 << x2;
                y2 |= b;

                if (o & b) == b {
                    break;
                }

                if x2 == 0 {
                    break;
                }

                x2 -= 1;
            }

            let mut x2 = f + 1;
            while x2 < 8 {
                let b = 1 << x2;
                y2 |= b;

                if (o & b) == b {
                    break;
                }

                x2 += 1;
            }

            rank_attack[sq as usize * 8 + f] = y2;
            f += 1;
        }

        sq += 1;
    }

    rank_attack
}

const RANK_ATTACKS: [u64; 512] = get_rank_attacks();

/*from rank1 to rank8*/
const RankMasks8: [u64; 8] = [
    0xFF,
    0xFF00,
    0xFF0000,
    0xFF000000,
    0xFF00000000,
    0xFF0000000000,
    0xFF000000000000,
    0xFF00000000000000,
];

const FileMasks8: [u64; 8] = [
    0x101010101010101,
    0x202020202020202,
    0x404040404040404,
    0x808080808080808,
    0x1010101010101010,
    0x2020202020202020,
    0x4040404040404040,
    0x8080808080808080,
];

/// Index with file + rank
const DiagonalMasks8: [u64; 15] = [
    0x1,
    0x102,
    0x10204,
    0x1020408,
    0x102040810,
    0x10204081020,
    0x1020408102040,
    0x102040810204080,
    0x204081020408000,
    0x408102040800000,
    0x810204080000000,
    0x1020408000000000,
    0x2040800000000000,
    0x4080000000000000,
    0x8000000000000000,
];

/// Index with file + rank
const AntiDiagonalMasks8: [u64; 15] = [
    0x80,
    0x8040,
    0x804020,
    0x80402010,
    0x8040201008,
    0x804020100804,
    0x80402010080402,
    0x8040201008040201,
    0x4020100804020100,
    0x2010080402010000,
    0x1008040201000000,
    0x804020100000000,
    0x402010000000000,
    0x201000000000000,
    0x100000000000000,
];

/*
static long HAndVMoves(int s) {
    //REMINDER: requires OCCUPIED to be up to date
    long binaryS=1L<<s;
    long possibilitiesHorizontal = (OCCUPIED - 2 * binaryS) ^ bit_swap(bit_swap(OCCUPIED) - 2 * bit_swap(binaryS));
    long possibilitiesVertical = ((OCCUPIED&FileMasks8[s % 8]) - (2 * binaryS)) ^ bit_swap(bit_swap(OCCUPIED&FileMasks8[s % 8]) - (2 * bit_swap(binaryS)));
    return (possibilitiesHorizontal&RankMasks8[s / 8]) | (possibilitiesVertical&FileMasks8[s % 8]);
}
static long DAndAntiDMoves(int s) {
    //REMINDER: requires OCCUPIED to be up to date
    long binaryS=1L<<s;
    long possibilitiesDiagonal = ((OCCUPIED&DiagonalMasks8[(s / 8) + (s % 8)]) - (2 * binaryS)) ^ bit_swap(bit_swap(OCCUPIED&DiagonalMasks8[(s / 8) + (s % 8)]) - (2 * bit_swap(binaryS)));
    long possibilitiesAntiDiagonal = ((OCCUPIED&AntiDiagonalMasks8[(s / 8) + 7 - (s % 8)]) - (2 * binaryS)) ^ bit_swap(bit_swap(OCCUPIED&AntiDiagonalMasks8[(s / 8) + 7 - (s % 8)]) - (2 * bit_swap(binaryS)));
    return (possibilitiesDiagonal&DiagonalMasks8[(s / 8) + (s % 8)]) | (possibilitiesAntiDiagonal&AntiDiagonalMasks8[(s / 8) + 7 - (s % 8)]);
} */
 */