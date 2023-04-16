use crate::{
    bitboard::BitBoard,
    board::Board,
    defs::{
        Piece, PieceType, Player, Score, Square, CASTLE_KING_FILES, CASTLE_QUEEN_FILES,
        CENTER_FILES, CENTER_SQUARES, EG_VALUE, MG_VALUE, PASSED_PAWN_SCORE,
    },
    gen::{
        attack::attacks,
        pesto::{EG_TABLE, MG_TABLE},
        tables::{CENTER_DISTANCE, DISTANCE, ISOLATED, PASSED},
    },
    movegen::pawn_caps,
};

const GAME_PHASE_INC: [Score; 6] = [0, 1, 1, 2, 4, 0];
const BISHOP_PAIR_BONUS: Score = 20;

pub fn evaluate(board: &Board) -> Score {
    // Score is from white's perspective
    let mut score = 0;
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
        score += mobility(board, piece, sq as Square);

        if piece.t == PieceType::Pawn {
            let pawn_score = match piece.c {
                Player::White => pawn_structure(piece.c, sq as Square, w_pawns, b_pawns),
                Player::Black => pawn_structure(piece.c, sq as Square, b_pawns, w_pawns),
            };

            score += pawn_score;
        }

        sq += 1;
    }

    mopup_eval(board, &mut eg);

    // Bishop pair bonus
    let w_bishops = board.player_piece_bb(Player::White, PieceType::Bishop);
    let b_bishops = board.player_piece_bb(Player::Black, PieceType::Bishop);

    if BitBoard::more_than_one(w_bishops) {
        score += BISHOP_PAIR_BONUS;
    }
    if BitBoard::more_than_one(b_bishops) {
        score -= BISHOP_PAIR_BONUS;
    }

    // undeveloped pieces penalty
    let w_knights = board.player_piece_bb(Player::White, PieceType::Knight);
    let b_knights = board.player_piece_bb(Player::Black, PieceType::Knight);
    mg[0] -= (BitBoard::count((w_knights | w_bishops) & BitBoard::RANK_1) * 8) as Score;
    mg[1] -= (BitBoard::count((b_knights | b_bishops) & BitBoard::RANK_8) * 8) as Score;

    // pawns controlling center of the board
    mg[0] += (BitBoard::count(w_pawns & CENTER_SQUARES) * 15) as Score;
    mg[1] += (BitBoard::count(b_pawns & CENTER_SQUARES) * 15) as Score;

    // pawn attacks
    let w_pawn_caps = pawn_caps(w_pawns, Player::White) & board.player_bb(Player::Black);
    let b_pawn_caps = pawn_caps(b_pawns, Player::Black) & board.player_bb(Player::White);

    mg[0] += (BitBoard::count(w_pawn_caps) * 3) as Score;
    mg[1] += (BitBoard::count(b_pawn_caps) * 3) as Score;

    // pawns defended by pawns
    let w_defenders = pawn_caps(w_pawns, Player::Black) & w_pawns;
    let b_defenders = pawn_caps(b_pawns, Player::White) & b_pawns;
    score += (BitBoard::count(w_defenders & w_pawns) * 4) as Score;
    score -= (BitBoard::count(b_defenders & b_pawns) * 4) as Score;

    // pawn shield for king safety
    king_pawn_shield(board, w_pawns, b_pawns, &mut mg);

    // tapered eval
    let turn = board.turn.as_usize();
    let opp = 1 - turn;

    let mg_score = mg[0] - mg[1];
    let eg_score = eg[0] - eg[1];
    let mg_phase = Score::min(24, game_phase);
    let eg_phase = 24 - mg_phase;

    score += (mg_score * mg_phase + eg_score * eg_phase) / 24;

    if board.turn == Player::White {
        score
    } else {
        -score
    }
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

// Structural evaluation of a piece, from white's perspective
fn mobility(board: &Board, piece: Piece, sq: Square) -> Score {
    if piece.t == PieceType::Pawn {
        return 0;
    }

    let occ = board.occ_bb();
    let my_bb = board.player_bb(piece.c);
    let opp_bb = occ & !my_bb;
    let mut score = 0;

    let moves = attacks(piece.t, sq, occ, piece.c);
    if moves == 0 {
        // penalize pieces that can't move
        score = -MG_VALUE[piece.t.as_usize()] / 15;
    } else {
        let open = BitBoard::count(moves & !occ);
        let att = BitBoard::count(moves & opp_bb);
        let def = BitBoard::count(moves & my_bb);

        // This score is in millipawns
        score = match piece.t {
            PieceType::Knight => 20 * open + 35 * att + 15 * def,
            PieceType::Bishop => 17 * open + 30 * att + 15 * def,
            PieceType::Rook => 15 * open + 20 * att + 15 * def,
            PieceType::Queen => 5 * open + 15 * att + 8 * def,
            PieceType::King => 4 * open + 15 * att + 10 * def,
            _ => panic!(),
        } as Score;

        score /= 30;
    }

    match piece.c {
        Player::White => score,
        _ => -score,
    }
}

// Structural evaluation of a pawn, from white's perspective
fn pawn_structure(side: Player, sq: Square, pawns: u64, opp_pawns: u64) -> Score {
    let mut score = 0;

    let file = sq % 8;
    // isolated pawn, as there are no pawns besides it
    if pawns & ISOLATED[file as usize] == 0 {
        score -= 20;
    }
    // doubled pawn
    if BitBoard::more_than_one(pawns & BitBoard::file_bb(sq)) {
        score -= 30;
    }

    // passed pawn
    if PASSED[side.as_usize()][sq as usize] & opp_pawns == 0 {
        let rel_rank = match side {
            Player::White => (sq / 8) as usize,
            Player::Black => (7 - sq / 8) as usize,
        };
        score += PASSED_PAWN_SCORE[rel_rank];
    }

    match side {
        Player::White => score,
        _ => -score,
    }
}

fn king_pawn_shield(board: &Board, w_pawns: u64, b_pawns: u64, mg: &mut [Score; 2]) {
    let w_king_bb = board.player_piece_bb(Player::White, PieceType::King);
    let b_king_bb = board.player_piece_bb(Player::Black, PieceType::King);

    let w_king_sq = BitBoard::bit_scan_forward(w_king_bb);
    let b_king_sq = BitBoard::bit_scan_forward(b_king_bb);

    // punish king in centre
    if w_king_bb & CENTER_FILES != 0 {
        mg[0] -= 25;
    }
    if b_king_bb & CENTER_FILES != 0 {
        mg[1] -= 25;
    }

    // punish king on open or semi-open file
    if (w_pawns | b_pawns) & BitBoard::file_bb(w_king_sq) == 0 {
        mg[0] -= 35;
    } else if w_pawns & BitBoard::file_bb(w_king_sq) == 0 {
        mg[0] -= 20;
    }
    if (w_pawns | b_pawns) & BitBoard::file_bb(b_king_sq) == 0 {
        mg[1] -= 35;
    } else if b_pawns & BitBoard::file_bb(b_king_sq) == 0 {
        mg[1] -= 20;
    }

    // If the king has wandered this far from home, he must have a reason to do so,
    // so don't evaluate a pawn shield
    if w_king_sq < 16 {
        // white king side
        if w_king_bb & (BitBoard::FILE_G | BitBoard::FILE_H) != 0 {
            mg[0] +=
                (BitBoard::count(w_pawns & CASTLE_KING_FILES & BitBoard::RANK_2) * 10) as Score;
            mg[0] += (BitBoard::count(w_pawns & CASTLE_KING_FILES & BitBoard::RANK_3) * 3) as Score;

            // punish empty file close to king
            for file in [BitBoard::FILE_G, BitBoard::FILE_H] {
                if file & w_pawns == 0 {
                    mg[0] -= 25;
                }
            }
        }
        // white queen side
        else if w_king_bb & CASTLE_QUEEN_FILES != 0 {
            mg[0] +=
                (BitBoard::count(w_pawns & CASTLE_QUEEN_FILES & BitBoard::RANK_2) * 10) as Score;
            mg[0] +=
                (BitBoard::count(w_pawns & CASTLE_QUEEN_FILES & BitBoard::RANK_3) * 3) as Score;

            // punish empty file close to king
            for file in [BitBoard::FILE_A, BitBoard::FILE_B, BitBoard::FILE_C] {
                if file & w_pawns == 0 {
                    mg[0] -= 25;
                }
            }
        }
        // Not castled yet
        else {
            mg[0] -= 15;
        }
    }

    // If the king has wandered this far from home, he must have a reason to do so,
    // so don't evaluate a pawn shield
    if b_king_sq > 47 {
        // black king side
        if b_king_bb & (BitBoard::FILE_G | BitBoard::FILE_H) != 0 {
            mg[1] +=
                (BitBoard::count(b_pawns & CASTLE_KING_FILES & BitBoard::RANK_7) * 10) as Score;
            mg[1] += (BitBoard::count(b_pawns & CASTLE_KING_FILES & BitBoard::RANK_6) * 3) as Score;

            // punish empty file close to king
            for file in [BitBoard::FILE_G, BitBoard::FILE_H] {
                if file & b_pawns == 0 {
                    mg[1] -= 25;
                }
            }
        }
        // black queen side
        else if b_king_bb & CASTLE_QUEEN_FILES != 0 {
            mg[1] +=
                (BitBoard::count(b_pawns & CASTLE_QUEEN_FILES & BitBoard::RANK_7) * 10) as Score;
            mg[1] +=
                (BitBoard::count(b_pawns & CASTLE_QUEEN_FILES & BitBoard::RANK_6) * 3) as Score;

            // punish empty file close to king
            for file in [BitBoard::FILE_A, BitBoard::FILE_B, BitBoard::FILE_C] {
                if file & b_pawns == 0 {
                    mg[1] -= 25;
                }
            }
        }
        // Not castled yet
        else {
            mg[1] -= 15;
        }
    }
}
