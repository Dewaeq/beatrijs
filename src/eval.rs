use crate::{
    board::Board,
    defs::{Piece, PieceType, Player, Score, Square, EG_VALUE, PASSED_PAWN_SCORE},
    gen::{
        attack::attacks,
        pesto::{EG_TABLE, MG_TABLE},
        tables::{CENTER_DISTANCE, DISTANCE, ISOLATED, PASSED},
    },
    movegen::pawn_caps, bitboard::BitBoard,
};

const GAME_PHASE_INC: [Score; 6] = [0, 1, 1, 2, 4, 0];
const BISHOP_PAIR_BONUS: Score = 20;

/// see https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function
pub fn evaluate(board: &Board) -> Score {
    let mut mg = [0; 2];
    let mut eg = [0; 2];
    let mut game_phase = 0;

    let w_pawns = board.player_piece_bb(Player::White, PieceType::Pawn);
    let b_pawns = board.player_piece_bb(Player::Black, PieceType::Pawn);

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
        if piece.t == PieceType::Pawn {
            mg[idx] += match piece.c {
                Player::White => pawn_structure(piece.c, sq as Square, w_pawns, b_pawns),
                Player::Black => pawn_structure(piece.c, sq as Square, b_pawns, w_pawns),
            };
        }

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

    // undeveloped pieces penalty
    let w_knights = board.player_piece_bb(Player::White, PieceType::Knight);
    let b_knights = board.player_piece_bb(Player::Black, PieceType::Knight);
    mg[0] -= (BitBoard::count((w_knights | w_bishops) & BitBoard::RANK_1) * 5) as Score;
    mg[1] -= (BitBoard::count((b_knights | b_bishops) & BitBoard::RANK_8) * 5) as Score;

    // pawn attacks
    let w_pawn_caps = pawn_caps(
        board.player_piece_bb(Player::White, PieceType::Pawn),
        Player::White,
    ) & board.player_bb(Player::Black);
    let b_pawn_caps = pawn_caps(
        board.player_piece_bb(Player::Black, PieceType::Pawn),
        Player::Black,
    ) & board.player_bb(Player::White);

    mg[0] += (BitBoard::count(w_pawn_caps) * 3) as Score;
    mg[1] += (BitBoard::count(b_pawn_caps) * 3) as Score;

    // pawns defended by pawns
    let w_defenders = pawn_caps(w_pawns, Player::Black);
    let b_defenders = pawn_caps(b_pawns, Player::White);
    mg[0] += (BitBoard::count(w_defenders & w_pawns) * 2) as Score;
    mg[1] += (BitBoard::count(b_defenders & b_pawns) * 2) as Score;

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
    if piece.t == PieceType::Pawn {
        return 0;
    }

    let occ = board.occ_bb();
    let my_bb = board.player_bb(board.turn);
    let opp_bb = occ & !my_bb;

    let moves = attacks(piece.t, sq, board.occ_bb(), piece.c);

    let open = BitBoard::count(moves & !occ);
    let att = BitBoard::count(moves & opp_bb);
    let def = BitBoard::count(moves & my_bb);

    // This score is in millipawns
    let score = match piece.t {
        PieceType::Knight => 20 * open + 35 * att + 15 * def,
        PieceType::Bishop => 17 * open + 30 * att + 15 * def,
        PieceType::Rook => 15 * open + 20 * att + 15 * def,
        PieceType::Queen => 5 * open + 15 * att + 8 * def,
        PieceType::King => 4 * open + 15 * att + 10 * def,
        _ => panic!(),
    };

    (score / 30) as Score
}

const fn pawn_structure(side: Player, sq: Square, pawns: u64, opp_pawns: u64) -> Score {
    let mut score = 0;

    let file = sq % 8;
    // isolated pawn, as there are no pawns besides it
    if pawns & ISOLATED[file as usize] == 0 {
        score -= 10;
    }
    // doubled pawn
    if BitBoard::more_than_one(pawns & BitBoard::file_bb(sq)) {
        score -= 15;
    }

    // passed pawn
    if PASSED[side.as_usize()][sq as usize] & opp_pawns == 0 {
        let rel_rank = match side {
            Player::White => (sq / 8) as usize,
            Player::Black => (7 - sq / 8) as usize,
        };
        score += PASSED_PAWN_SCORE[rel_rank];
    }

    score
}
