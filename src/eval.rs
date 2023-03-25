use crate::{
    bitboard::BitBoard,
    board::Board,
    defs::{PieceType, Player, Score},
    gen::pesto::{EG_TABLE, MG_TABLE},
};

const GAME_PHASE_INC: [Score; 6] = [0, 1, 1, 2, 4, 0];

/// see https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function
pub fn evaluate(board: &Board) -> Score {
    let mut mg = [0; 2];
    let mut eg = [0; 2];
    let mut game_phase = 0;

    let mut sq = 0;
    for piece in board.pieces {
        if piece.is_none() {
            sq += 1;
            continue;
        }

        let idx = piece.c.as_usize();
        mg[idx] += MG_TABLE[piece.as_usize()][sq];
        eg[idx] += EG_TABLE[piece.as_usize()][sq];
        game_phase += GAME_PHASE_INC[piece.t.as_usize()];

        sq += 1;
    }

    // tapered eval
    let turn = board.turn.as_usize();
    let opp = 1 - turn;

    let mg_score = mg[turn] - mg[opp];
    let eg_score = eg[turn] - eg[opp];
    let mg_phase = Score::min(24, game_phase);
    let eg_phase = 24 - mg_phase;

    (mg_score * mg_phase + eg_score * eg_phase) / 24
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
