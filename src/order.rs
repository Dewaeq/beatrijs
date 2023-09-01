use crate::{
    bitboard::BitBoard,
    bitmove::BitMove,
    board::Board,
    defs::{PieceType, Score},
    gen::eval::MVV_LVA,
    movegen::pawn_caps,
    movelist::{self, MoveList},
    search::HistoryTable,
};

const HASH_BONUS: i32 = 8_000_000;
const WINNING_CAPTURE_BONUS: i32 = 7_000_000;
const PROMOTE_BONUS: i32 = 6_000_000;
const LOSING_CAPTURE_BONUS: i32 = 5_000_000;
const KILLER_1_BONUS: i32 = 900_000;
const KILLER_2_BONUS: i32 = 800_000;

pub const fn score_move(
    m: u16,
    board: &Board,
    history_table: &HistoryTable,
    hash_move: u16,
) -> Score {
    if m == hash_move {
        return HASH_BONUS;
    }

    let opp_pawn_attacks = 0;

    let (src, dest) = BitMove::from_to(m);
    let move_piece = board.piece_type(src);
    let mut score = 0;

    if BitMove::is_cap(m) {
        let cap_piece = if BitMove::is_ep(m) {
            PieceType::Pawn
        } else {
            board.piece_type(dest)
        };

        let delta = MVV_LVA[move_piece.as_usize()][cap_piece.as_usize()];
        let is_winning = delta >= 0;
        let can_recapture = BitBoard::contains(opp_pawn_attacks, dest);

        if !is_winning && can_recapture {
            score += LOSING_CAPTURE_BONUS + delta;
        } else {
            score += WINNING_CAPTURE_BONUS + delta;
        }
    }
    // Quiet move
    else {
        score += history_table[board.turn.as_usize()][src as usize][dest as usize];

        let ply = board.pos.ply;
        if m == board.killers[0][ply] {
            score += KILLER_1_BONUS;
        } else if m == board.killers[1][ply] {
            score += KILLER_2_BONUS;
        }
    }

    if BitMove::is_prom(m) {
        let flag = BitMove::flag(m);
        score += PROMOTE_BONUS + BitMove::prom_type(flag).mg_value();
    } else if !matches!(move_piece, PieceType::King) {
        if BitBoard::contains(opp_pawn_attacks, dest) {
            score -= 40;
        }
    }

    score
}

pub fn sort_moves(
    move_list: &mut MoveList,
    board: &Board,
    hash_move: u16,
    history_table: &HistoryTable,
) {
    let opp_pawns = board.player_piece_bb(board.turn.opp(), PieceType::Pawn);
    let opp_pawn_attacks = pawn_caps(opp_pawns, board.turn.opp());

    for i in 0..move_list.size() {
        let m = move_list.get(i);

        if m == hash_move {
            move_list.set_score(i, HASH_BONUS);
            continue;
        }

        let (src, dest) = BitMove::from_to(m);
        let move_piece = board.piece_type(src);
        let mut score = 0;

        if BitMove::is_cap(m) {
            let cap_piece = if BitMove::is_ep(m) {
                PieceType::Pawn
            } else {
                board.piece_type(dest)
            };

            let delta = MVV_LVA[move_piece.as_usize()][cap_piece.as_usize()];
            let is_winning = delta >= 0;
            let can_recapture = BitBoard::contains(opp_pawn_attacks, dest);

            if !is_winning && can_recapture {
                score += LOSING_CAPTURE_BONUS + delta;
            } else {
                score += WINNING_CAPTURE_BONUS + delta;
            }
        }
        // Quiet move
        else {
            score += history_table[board.turn.as_usize()][src as usize][dest as usize];

            let ply = board.pos.ply;
            if m == board.killers[0][ply] {
                score += KILLER_1_BONUS;
            } else if m == board.killers[1][ply] {
                score += KILLER_2_BONUS;
            }
        }

        if BitMove::is_prom(m) {
            let flag = BitMove::flag(m);
            score += PROMOTE_BONUS + BitMove::prom_type(flag).mg_value();
        } else if move_piece != PieceType::King {
            if BitBoard::contains(opp_pawn_attacks, dest) {
                score -= 40;
            }
        }

        move_list.set_score(i, score);
    }

    let size = move_list.size();
    sort(&mut move_list.entries[..size]);
}

pub fn sort(ar: &mut [(u16, i32)]) {
    match ar.len() {
        0 | 1 => return,
        2 => {
            if ar[0].1 < ar[1].1 {
                ar.swap(0, 1);
            }
            return;
        }
        _ => (),
    }

    let (pivot, slice) = ar.split_last_mut().unwrap();
    // Everything before left is >= pivot
    let mut left: usize = 0;
    // Everything after right is < pivot
    let mut right: usize = slice.len() - 1;

    while left <= right {
        if slice[left].1 >= pivot.1 {
            left += 1;
        } else if slice[right].1 < pivot.1 {
            if right == 0 {
                break;
            }
            right -= 1;
        } else {
            slice.swap(left, right);
            left += 1;
            right -= 1;
        }
    }

    // Move the pivot to the correct position
    ar.swap(ar.len() - 1, left);

    sort(&mut ar[..left]);
    sort(&mut ar[right..]);
}

pub fn pick_next_move(move_list: &mut MoveList, move_num: usize) {
    let mut best_score = 0;
    let mut best_index = move_num;

    for index in move_num..move_list.size() {
        if move_list.get_score(index) > best_score {
            best_score = move_list.get_score(index);
            best_index = index;
        }
    }

    move_list.swap(move_num, best_index);
}
