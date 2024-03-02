use crate::bitmove::BitMove;
use crate::board::Board;
use crate::defs::{Depth, PieceType, Player, Score};
use crate::search::{IS_MATE, MATE};
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

/// # Arguments
///
/// * `elapsed` - Elapsed time from the start of the search, in milliseconds
pub fn print_search_info(
    depth: Depth,
    sel_depth: usize,
    score: Score,
    elapsed: f64,
    num_nodes: u64,
    hash_full: usize,
    pv: &[u16],
    turn: Player,
) {
    let score_str = if score.abs() == MATE {
        format!("mate",)
    } else if score > IS_MATE {
        format!("mate {}", (MATE - score + 1) / 2 as Score)
    } else if score < -IS_MATE {
        format!("mate {}", -(score + MATE) / 2 as Score)
    } else {
        format!("cp {score}")
    };

    print!(
        "info depth {} seldepth {} score {} nodes {} time {} nps {} hashfull {} ",
        depth,
        sel_depth,
        score_str,
        num_nodes,
        elapsed as u64,
        (num_nodes as f64 / elapsed * 1000f64) as u64,
        hash_full,
    );
    print_pv(&pv);
}

pub fn print_pv(pv: &[u16]) {
    print!("pv ");
    for &m in pv {
        if m == 0 {
            break;
        }
        print!("{} ", BitMove::pretty_move(m));
    }

    println!();
}

pub const fn mirror(sq: Square) -> Square {
    sq ^ 56
}

pub const fn is_draw(board: &Board) -> bool {
    board.pos.half_move_count >= 100 || is_repetition(board) || is_material_draw(board)
}

pub const fn is_repetition(board: &Board) -> bool {
    let mut i = board.history.count as i32 - 2;
    while i >= 0 && i >= board.history.count as i32 - board.pos.half_move_count as i32 {
        if board.history.get_key(i as usize) == board.key() {
            return true;
        }

        i -= 2;
    }

    return false;

    /*board
    .history
    .iter()
    .take(board.history.count)
    .rev()
    .take(board.pos.half_move_count as usize)
    .skip(1)
    .step_by(2)
    .any(|pos| pos.key == board.key())*/
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
    let bishops = board.piece_bb(PieceType::Bishop);

    // KvN, KvNN and KvB are draws
    if (only_white_king || only_black_king)
        && ((num_knights <= 2 && bishops == 0)
            || (num_knights == 0 && !BitBoard::more_than_one(bishops)))
    {
        return true;
    }

    return false;
}

pub const fn ranks_in_front_of(side: Player, sq: Square) -> u64 {
    let bb = BitBoard::rank_bb(sq);
    front_span(side, bb)
}

pub const fn front_span(side: Player, bb: u64) -> u64 {
    match side {
        Player::White => north_one(north_fill(bb)),
        Player::Black => south_one(south_fill(bb)),
    }
}

pub const fn fill_up(side: Player, bb: u64) -> u64 {
    match side {
        Player::White => north_fill(bb),
        Player::Black => south_fill(bb),
    }
}

pub const fn fill_down(side: Player, bb: u64) -> u64 {
    match side {
        Player::White => south_fill(bb),
        Player::Black => north_fill(bb),
    }
}

pub const fn north_fill(mut bb: u64) -> u64 {
    bb |= bb << 8;
    bb |= bb << 16;
    bb |= bb << 32;

    bb
}

pub const fn south_fill(mut bb: u64) -> u64 {
    bb |= bb >> 8;
    bb |= bb >> 16;
    bb |= bb >> 32;

    bb
}

pub const fn file_fill(bb: u64) -> u64 {
    north_fill(bb) | south_fill(bb)
}

pub const fn north_one(bb: u64) -> u64 {
    bb << 8
}

pub const fn south_one(bb: u64) -> u64 {
    bb >> 8
}

pub const fn east_one(bb: u64) -> u64 {
    (bb & !BitBoard::FILE_H) << 1
}

pub const fn west_one(bb: u64) -> u64 {
    (bb & !BitBoard::FILE_A) >> 1
}
