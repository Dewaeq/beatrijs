use crate::{
    defs::{PieceType, Square},
    utils::coord_from_square,
};

/// Move encoded into a `u16`
///
/// Bits 0-5 are for the source square,
///
/// Bits 6-11 are for the destination square,
///
/// Bits 12-15 are flags
pub struct BitMove;

impl BitMove {
    pub const fn from_squares(src: Square, dest: Square) -> u16 {
        src as u16 | ((dest as u16) << 6)
    }

    pub const fn from_flag(src: Square, dest: Square, flag: u8) -> u16 {
        BitMove::from_squares(src, dest) | ((flag as u16) << 12)
    }

    pub const fn src(bitmove: u16) -> Square {
        (bitmove & 0b111111) as Square
    }

    pub const fn dest(bitmove: u16) -> Square {
        (bitmove >> 6 & 0b111111) as Square
    }

    pub const fn flag(bitmove: u16) -> u8 {
        (bitmove >> 12) as u8
    }

    pub const fn is_cap(bitmove: u16) -> bool {
        BitMove::flag(bitmove) & 0b0100 != 0
    }

    pub const fn is_prom(bitmove: u16) -> bool {
        BitMove::flag(bitmove) & 0b1000 != 0
    }

    pub const fn is_ep(bitmove: u16) -> bool {
        BitMove::flag(bitmove) == MoveFlag::EN_PASSANT
    }

    pub const fn is_castle(bitmove: u16) -> bool {
        BitMove::flag(bitmove) == MoveFlag::CASTLE_KING
            || BitMove::flag(bitmove) == MoveFlag::CASTLE_QUEEN
    }

    #[allow(unused)]
    pub const fn is_tactical(bitmove: u16) -> bool {
        BitMove::is_cap(bitmove) || BitMove::is_prom(bitmove)
    }

    /// No capture, promotion, en passant or castle
    pub const fn is_normal(bitmove: u16) -> bool {
        bitmove & 0b1110 == 0
    }

    pub const fn prom_type(flag: u8) -> PieceType {
        // Remove capture bit
        match flag & 0b1011 {
            MoveFlag::PROMOTE_KNIGHT => PieceType::Knight,
            MoveFlag::PROMOTE_BISHOP => PieceType::Bishop,
            MoveFlag::PROMOTE_ROOK => PieceType::Rook,
            MoveFlag::PROMOTE_QUEEN => PieceType::Queen,
            _ => PieceType::None,
        }
    }

    #[allow(dead_code)]
    pub fn print_move(bitmove: u16) {
        let src = BitMove::src(bitmove);
        let dest = BitMove::dest(bitmove);
        let flag = BitMove::flag(bitmove);

        println!("from: {src} to: {dest} flag: {flag}");
    }

    pub fn pretty_move(bitmove: u16) -> String {
        if bitmove == 0 {
            return "null".to_owned();
        }

        fn file_idx_to_char(file: Square) -> String {
            match file {
                0 => "a".to_owned(),
                1 => "b".to_owned(),
                2 => "c".to_owned(),
                3 => "d".to_owned(),
                4 => "e".to_owned(),
                5 => "f".to_owned(),
                6 => "g".to_owned(),
                7 => "h".to_owned(),
                _ => "".to_owned(),
            }
        }

        let src = BitMove::src(bitmove);
        let dest = BitMove::dest(bitmove);

        let (src_x, src_y) = coord_from_square(src);
        let (dest_x, dest_y) = coord_from_square(dest);

        let mut result = format!("{}{}", file_idx_to_char(src_x), src_y + 1);
        let dest_str = format!("{}{}", file_idx_to_char(dest_x), dest_y + 1);
        result.push_str(&dest_str);

        if BitMove::is_prom(bitmove) {
            let prom_str = match BitMove::prom_type(BitMove::flag(bitmove)) {
                PieceType::Knight => "n",
                PieceType::Bishop => "b",
                PieceType::Rook => "r",
                PieceType::Queen => "q",
                _ => "",
            };
            result.push_str(prom_str);
        }

        result
    }
}

/// Bits 0-1 are special flags
///
/// Bit 2 defines a capture
///
/// Bit 3 defines a promotion
///
/// See <https://www.chessprogramming.org/Encoding_Moves#From-To_Based>
pub struct MoveFlag;

#[allow(dead_code)]
impl MoveFlag {
    pub const QUIET: u8 = 0;
    pub const DOUBLE_PAWN_PUSH: u8 = 1;
    pub const CASTLE: u8 = 2;
    pub const CASTLE_KING: u8 = 2;
    pub const CASTLE_QUEEN: u8 = 3;
    pub const CAPTURE: u8 = 4;
    pub const EN_PASSANT: u8 = 5;
    pub const PROMOTE: u8 = 8;
    pub const PROMOTE_KNIGHT: u8 = 8;
    pub const PROMOTE_BISHOP: u8 = 9;
    pub const PROMOTE_ROOK: u8 = 10;
    pub const PROMOTE_QUEEN: u8 = 11;
    pub const PROMOTE_KNIGHT_CAPTURE: u8 = 12;
    pub const PROMOTE_BISHOP_CAPTURE: u8 = 13;
    pub const PROMOTE_ROOK_CAPTURE: u8 = 14;
    pub const PROMOTE_QUEEN_CAPTURE: u8 = 15;
}
