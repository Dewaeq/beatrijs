use crate::defs::{PieceType, Player, Square};

include!(concat!(env!("OUT_DIR"), "/zobrist.rs"));

pub struct Zobrist;

impl Zobrist {
    pub const fn piece(side: Player, piece: PieceType, sq: Square) -> u64 {
        PIECES[piece.as_usize() + side.as_usize() * 6][sq as usize]
    }

    pub const fn side() -> u64 {
        SIDE
    }

    pub const fn castle(castling: u8) -> u64 {
        CASTLE[castling as usize]
    }

    pub const fn ep(ep_file: Square) -> u64 {
        EP[ep_file as usize]
    }
}
