use crate::{
    bitmove::BitMove,
    board::Board,
    defs::{Value, MAX_MOVES},
    gen::eval::MVV_LVA,
    movelist::MoveList,
};

pub fn order_moves(moves: &mut MoveList, board: &Board) {
    for i in 0..moves.size() {
        let m = moves.get(i);

        if BitMove::is_ep(m) {
            moves.set_score(i, 105);
        } else if BitMove::is_cap(m) {
            let move_piece = board.piece_type(BitMove::src(m));
            let cap_piece = board.piece_type(BitMove::dest(m));

            moves.set_score(i, MVV_LVA[move_piece.as_usize()][cap_piece.as_usize()]);
        }
    }

    sort_moves(moves)
}

pub fn order_quiets(moves: &mut MoveList, board: &Board) {
    for i in 0..moves.size() {
        let m = moves.get(i);
        let s = board.see_capture(m);
        moves.set_score(i, board.see_capture(m));
    }

    sort_moves(moves)
}

/// Selection sort
fn sort_moves(moves: &mut MoveList) {
    for i in 0..moves.size() - 1 {
        let mut j = i + 1;
        while j > 0 {
            let swap_index = j - 1;
            if moves.get_score(swap_index) < moves.get_score(j) {
                moves.swap(j, swap_index);
            }

            j -= 1;
        }
    }
}
