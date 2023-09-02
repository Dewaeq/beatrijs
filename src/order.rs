use crate::{
    bitboard::BitBoard,
    bitmove::BitMove,
    board::Board,
    defs::{PieceType, Score},
    gen::eval::MVV_LVA,
    movelist::MoveList,
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
