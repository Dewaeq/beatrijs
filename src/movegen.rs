use crate::{
    bitboard::BitBoard,
    bitmove::{BitMove, MoveFlag},
    color::Color,
    defs::{GenType, PieceType, Square},
    gen::{
        attack::{
            attacks, bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks,
        },
        between::between,
        eval::MVV_LVA,
    },
    movelist::MoveList,
    search::HistoryTable,
    utils::adjacent_files,
};

const HASH_BONUS: i32 = 9_000_000;
const PROMOTE_BONUS: i32 = 7_000_000;
const GOOD_CAPTURE_BONUS: i32 = 6_000_000;
const KILLER_1_BONUS: i32 = 5_000_000;
const KILLER_2_BONUS: i32 = 4_000_000;
const BAD_CAPTURE_BONUS: i32 = 3_000_000;

#[inline]
pub const fn pawn_push(pawns: u64, color: Color) -> u64 {
    match color {
        Color::White => pawns << 8,
        Color::Black => pawns >> 8,
    }
}

#[inline]
const fn double_pawn_push(pawns: u64, color: Color) -> u64 {
    match color {
        Color::White => pawns << 16,
        Color::Black => pawns >> 16,
    }
}

#[inline]
const fn pawn_cap_east(pawns: u64, color: Color) -> u64 {
    match color {
        Color::White => (pawns & !BitBoard::FILE_H) << 9,
        Color::Black => (pawns & !BitBoard::FILE_H) >> 7,
    }
}

#[inline]
const fn pawn_cap_west(pawns: u64, color: Color) -> u64 {
    match color {
        Color::White => (pawns & !BitBoard::FILE_A) << 7,
        Color::Black => (pawns & !BitBoard::FILE_A) >> 9,
    }
}

#[inline]
pub const fn pawn_caps(pawns: u64, color: Color) -> u64 {
    pawn_cap_west(pawns, color) | pawn_cap_east(pawns, color)
}
