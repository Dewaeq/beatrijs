// use super::Board;

// pub fn make_null_move(board: &mut Board) {
//     board.history.push(board.position);
//     board.turn = board.turn.opponent();
//     board.position.ply += 1;
//     board.position.rule_fifty += 1;
// }


// pub fn undo_null_move(board: &mut Board) {
//     board.position = board.history.get_current();
//     board.turn = board.turn.opponent();
//     board.history.pop();
// }
