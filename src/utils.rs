use crate::bitmove::BitMove;
use crate::defs::Score;
use crate::search::{IMMEDIATE_MATE_SCORE, IS_MATE};
use crate::{bitboard::BitBoard, defs::Square};

pub fn square_from_string(str: &str) -> Square {
    assert!(str.len() == 2);

    let file = (str.as_bytes()[0] - 97);
    let rank = (str.as_bytes()[1] - 49);

    (rank as Square) * 8 + (file as Square)
}

pub fn square_to_string(sq: Square) -> String {
    if !is_in_board(sq) {
        return "".to_owned();
    }

    let (file, rank) = coord_from_square(sq);
    let file_str = char::from_u32(file as u32 + 97).unwrap();
    let rank_str = char::from_u32(rank as u32 + 49).unwrap();

    format!("{file_str}{rank_str}")
}

pub const fn adjacent_files(file: Square) -> u64 {
    if file == 0 {
        BitBoard::file_bb(file + 1)
    } else if file == 7 {
        BitBoard::file_bb(file - 1)
    } else {
        BitBoard::file_bb(file - 1) | BitBoard::file_bb(file + 1)
    }
}

/// Returns `(file, rank)`
pub const fn coord_from_square(sq: Square) -> (Square, Square) {
    (sq % 8, sq / 8)
}

pub const fn is_in_board(square: Square) -> bool {
    // u8 will never be negative, so we can skip that check
    square < 64 && square >= 0
}

/// `const` alternative to [`std::cmp::min`]
pub const fn b_min(a: Square, b: Square) -> Square {
    if a > b {
        b
    } else {
        a
    }
}

/// `const` alternative to [`std::cmp::max`]
pub const fn b_max(a: Square, b: Square) -> Square {
    if a > b {
        a
    } else {
        b
    }
}

pub fn print_search_info(
    depth: u8,
    score: Score,
    total_time: u64,
    search_time: f64,
    best_move: u16,
    num_nodes: u64,
    pv: &[u16],
) {
    let score_str = if score > IS_MATE {
        format!("mate {}", (score - IMMEDIATE_MATE_SCORE) / 2)
    } else if score < -IS_MATE {
        format!("mate {}", (score + IMMEDIATE_MATE_SCORE) / 2)
    } else {
        format!("cp {score}")
    };

    print!(
        "info depth {} score {} nodes {} time {} nps {}",
        depth,
        score_str,
        num_nodes,
        total_time,
        (num_nodes as f64 / search_time) as u64
    );
    print_pv(&pv);
}

pub fn print_pv(pv: &[u16]) {
    print!(" pv ");
    for &m in pv {
        if m == 0 {
            break;
        }
        print!("{} ", BitMove::pretty_move(m));
    }

    println!();
}

pub const fn mirror(sq: Square) -> Square {
    unsafe { *MIRRORED.get_unchecked(sq as usize) }
}

#[rustfmt::skip]
const MIRRORED: [Square; 64] = [
    56, 57, 58, 59, 60, 61, 62, 63,
    48, 49, 50, 51, 52, 53, 54, 55,
    40, 41, 42, 43, 44, 45, 46, 47,
    32, 33, 34, 35, 36, 37, 38, 39,
    24, 25, 26, 27, 28, 29, 30, 31,
    16, 17, 18, 19, 20, 21, 22, 23,
     8,  9, 10, 11, 12, 13, 14, 15,
     0,  1,  2,  3,  4,  5,  6,  7,
];
