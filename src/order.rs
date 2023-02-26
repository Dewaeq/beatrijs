use crate::{
    bitmove::BitMove,
    board::Board,
    defs::{Value, MAX_MOVES},
    gen::eval::MVV_LVA,
    movelist::MoveList,
};

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
