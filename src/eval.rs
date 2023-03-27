use crate::{
    bitboard::BitBoard,
    board::Board,
    defs::{Piece, PieceType, Player, Score, Square, EG_VALUE},
    gen::{
        attack::attacks,
        pesto::{EG_TABLE, MG_TABLE},
        tables::{CENTER_DISTANCE, DISTANCE},
    },
};

const GAME_PHASE_INC: [Score; 6] = [0, 1, 1, 2, 4, 0];
const BISHOP_PAIR_BONUS: Score = 20;

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

        mg[idx] += mobility(board, piece, sq as Square);

        sq += 1;
    }

    mopup_eval(board, &mut eg);

    // Bishop pair bonus
    let w_bishops = board.player_piece_bb(Player::White, PieceType::Bishop);
    let b_bishops = board.player_piece_bb(Player::Black, PieceType::Bishop);

    if BitBoard::more_than_one(w_bishops) {
        mg[0] += BISHOP_PAIR_BONUS;
    }
    if BitBoard::more_than_one(b_bishops) {
        mg[1] += BISHOP_PAIR_BONUS;
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

fn mopup_eval(board: &Board, eg: &mut [Score; 2]) {
    // Don't apply mop-up when there are still pawns on the board
    if board.piece_bb(PieceType::Pawn) != 0 {
        return;
    }

    // Only apply mopup when we're up on material,
    // require at least a rook
    let turn = board.turn.as_usize();
    let opp = 1 - turn;
    let diff = eg[turn] - eg[opp];
    if diff < EG_VALUE[3] - 100 {
        return;
    }

    let king_sq = board.cur_king_square() as usize;
    let opp_king_sq = board.king_square(board.turn.opp()) as usize;

    let center_dist = 4.7 * CENTER_DISTANCE[opp_king_sq] as f32;
    let kings_dist = 1.6 * (14 - DISTANCE[king_sq][opp_king_sq]) as f32;
    let mopup = (center_dist + kings_dist) as Score;

    eg[turn] += mopup;
}

fn mobility(board: &Board, piece: Piece, sq: Square) -> Score {
    if piece.t == PieceType::Pawn || piece.t == PieceType::King {
        return 0;
    }

    let moves = attacks(piece.t, sq, board.occ_bb(), piece.c);
    let count = BitBoard::count(moves) as Score;

    // TODO: find proper values, these are just my guesses and haven't been tested yet
    match piece.t {
        PieceType::Knight => 4 * count,
        PieceType::Bishop => 6 * count,
        PieceType::Rook => 5 * count,
        PieceType::Queen => 4 * count,
        _ => 0,
    }
}
