use crate::{
    bitmove::BitMove,
    board::Board,
    defs::{Value, MAX_MOVES},
    movelist::MoveList,
};

pub fn order_moves(moves: &mut MoveList, board: &Board) {
    let mut move_scores = [0; MAX_MOVES];

    for i in 0..moves.size() {
        let m = moves.get(i);

        if BitMove::is_cap(m) {
            let move_piece = board.piece_type(BitMove::src(m));
            let cap_piece = board.piece_type(BitMove::dest(m));

            move_scores[i] += 100 * Value::piece_value(cap_piece) - Value::piece_value(move_piece);
        }
    }

    sort_moves(moves, move_scores)
}

/// Selection sort
fn sort_moves(moves: &mut MoveList, mut move_scores: [i32; MAX_MOVES]) {
    for i in 0..moves.size() - 1 {
        let mut j = i + 1;
        while j > 0 {
            let swap_index = j - 1;
            if move_scores[swap_index] < move_scores[j] {
                moves.swap(j, swap_index);
            }

            j -= 1;
        }
    }
}
