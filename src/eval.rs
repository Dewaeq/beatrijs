use crate::{
    bitboard::BitBoard,
    board::Board,
    defs::{PieceType, Player, Score, Value},
    gen::pesto::MG_TABLE,
};

pub fn evaluate(board: &Board) -> Score {
    let mut white = 0;
    let mut black = 0;

    count_psqt(board, &mut white, &mut black);

    if board.turn == Player::White {
        white - black
    } else {
        black - white
    }
}

fn count_psqt(board: &Board, white: &mut Score, black: &mut Score) {
    let mut sq = 0;
    for piece in board.pieces {
        if piece.is_none() {
            sq += 1;
            continue;
        }

        let score = MG_TABLE[piece.as_usize()][sq];
        match piece.c {
            Player::White => *white += score,
            Player::Black => *black += score,
        }

        sq += 1;
    }
}
