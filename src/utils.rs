use crate::bitmove::BitMove;
use crate::board::Board;
use crate::defs::{PieceType, Player, Score};
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
    print!(
        "info depth {} move {} cp {} nodes {} time {} nps {}",
        depth,
        BitMove::pretty_move(best_move),
        score,
        num_nodes,
        total_time,
        (num_nodes as f64 / search_time) as u64
    );

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

pub const fn is_draw(board: &Board) -> bool {
    board.pos.rule_fifty >= 100 || is_repetition(board) || is_material_draw(board)
}

pub const fn is_repetition(board: &Board) -> bool {
    if board.pos.ply < 2 || board.pos.rule_fifty < 2 {
        return false;
    }

    let mut count = 0;
    let mut i = 1;

    while i <= (board.pos.rule_fifty + 1) as usize {
        let key = board.history.get_key(board.pos.ply - i);
        if key == board.key() {
            count += 1;
        }

        if count == 2 {
            return true;
        }

        i += 2;
    }

    false
}

const fn is_material_draw(board: &Board) -> bool {
    let only_white_king = BitBoard::only_one(board.player_bb(Player::White));
    let only_black_king = BitBoard::only_one(board.player_bb(Player::Black));

    if only_black_king && only_white_king {
        return true;
    }

    let pawns = board.piece_bb(PieceType::Pawn);
    if pawns != 0 {
        return false;
    }

    let rooks = board.piece_bb(PieceType::Rook);
    if rooks != 0 {
        return false;
    }

    let queens = board.piece_bb(PieceType::Queen);
    if queens != 0 {
        return false;
    }

    let num_knights = BitBoard::count(board.piece_bb(PieceType::Knight));
    let num_bishops = BitBoard::count(board.piece_bb(PieceType::Bishop));

    if (only_white_king || only_black_king)
        && ((num_knights < 2 && num_bishops == 0) || (num_knights == 0 && num_bishops < 2))
    {
        return true;
    }

    return false;
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
