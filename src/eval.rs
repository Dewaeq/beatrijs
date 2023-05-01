use crate::{
    bitboard::BitBoard,
    board::Board,
    defs::{
        Piece, PieceType, Player, Score, Square, CASTLE_KING_FILES, CASTLE_QUEEN_FILES,
        CENTER_SQUARES, EG_VALUE, MG_VALUE, PASSED_PAWN_SCORE,
    },
    gen::{
        attack::{attacks, king_attacks},
        pesto::{EG_TABLE, MG_TABLE},
        tables::{CENTER_DISTANCE, DISTANCE, ISOLATED, PASSED, SHIELDING_PAWNS},
    },
    movegen::pawn_caps,
    utils::ranks_in_front_of,
};

const GAME_PHASE_INC: [Score; 6] = [0, 1, 1, 2, 4, 0];
const BISHOP_PAIR_BONUS: Score = 7;

const SHIELD_MISSING: [i32; 4] = [-2, -23, -38, -55];
const SHIELD_MISSING_ON_OPEN_FILE: [i32; 4] = [-8, -10, -37, -66];

const SAFE_MASK: [u64; 2] = [
    (BitBoard::FILE_C | BitBoard::FILE_D | BitBoard::FILE_E | BitBoard::FILE_F)
        & (BitBoard::RANK_2 | BitBoard::RANK_3 | BitBoard::RANK_4),
    (BitBoard::FILE_C | BitBoard::FILE_D | BitBoard::FILE_E | BitBoard::FILE_F)
        & (BitBoard::RANK_5 | BitBoard::RANK_6 | BitBoard::RANK_7),
];

pub fn evaluate(board: &Board) -> Score {
    // Score is from white's perspective
    let mut score = 0;
    let mut mg = [0; 2];
    let mut eg = [0; 2];
    let mut piece_material = [0; 2];
    let mut pawn_material = [0; 2];
    let mut game_phase = 0;
    let mut attacked_by = AttackedBy::new();

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
        score += mobility(board, piece, sq as Square, &mut attacked_by);

        if piece.t == PieceType::Pawn {
            score += match piece.c {
                Player::White => pawn_structure(piece.c, sq as Square, w_pawns, b_pawns),
                Player::Black => pawn_structure(piece.c, sq as Square, b_pawns, w_pawns),
            };
            pawn_material[idx] += MG_VALUE[0];
        } else {
            piece_material[idx] += MG_VALUE[piece.t.as_usize()];
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
    mg[0] -= (BitBoard::count((w_knights | w_bishops) & BitBoard::RANK_1) * 6) as Score;
    mg[1] -= (BitBoard::count((b_knights | b_bishops) & BitBoard::RANK_8) * 6) as Score;

    // pawns controlling center of the board
    mg[0] += (BitBoard::count(w_pawns & CENTER_SQUARES) * 4) as Score;
    mg[1] += (BitBoard::count(b_pawns & CENTER_SQUARES) * 4) as Score;

    // pawn attacks
    let w_pawn_caps = pawn_caps(w_pawns, Player::White) & board.player_bb(Player::Black);
    let b_pawn_caps = pawn_caps(b_pawns, Player::Black) & board.player_bb(Player::White);

    attacked_by.w_pawns = w_pawn_caps;
    attacked_by.white |= w_pawn_caps;
    attacked_by.b_pawns = b_pawn_caps;
    attacked_by.black |= b_pawn_caps;

    mg[0] += (BitBoard::count(w_pawn_caps) * 3) as Score;
    mg[1] += (BitBoard::count(b_pawn_caps) * 3) as Score;

    // pawns defended by pawns
    let w_defenders = pawn_caps(w_pawns, Player::Black) & w_pawns;
    let b_defenders = pawn_caps(b_pawns, Player::White) & b_pawns;
    score += (BitBoard::count(w_defenders) * 4) as Score;
    score -= (BitBoard::count(b_defenders) * 4) as Score;

    // attacks on king
    let w_king_sq = board.king_square(Player::White);
    let b_king_sq = board.king_square(Player::Black);

    let w_king_bb = BitBoard::from_sq(w_king_sq);
    let b_king_bb = BitBoard::from_sq(b_king_sq);

    score -= (BitBoard::count(attacked_by.black & king_attacks(w_king_sq)) * 9) as Score;
    score -= (BitBoard::count(attacked_by.black & w_king_bb) * 16) as Score;

    score += (BitBoard::count(attacked_by.white & king_attacks(b_king_sq)) * 9) as Score;
    score += (BitBoard::count(attacked_by.white & b_king_bb) * 16) as Score;

    // pawn shield for king safety
    king_pawn_shield(
        board, w_pawns, b_pawns, &mut mg, w_king_sq, b_king_sq, w_king_bb, b_king_bb,
    );

    // Control of space on the player's side of the board
    let total_non_pawn = piece_material[0] + piece_material[1];
    score += eval_space(&board, Player::White, w_pawns, &attacked_by, total_non_pawn);
    score -= eval_space(&board, Player::Black, b_pawns, &attacked_by, total_non_pawn);

    // tapered eval
    let mg_score = mg[0] - mg[1];
    let eg_score = eg[0] - eg[1];
    let mg_phase = game_phase.min(24);
    let eg_phase = 24 - mg_phase;

    score += (mg_score * mg_phase + eg_score * eg_phase) / 24;

    let (stronger, weaker) = if score > 0 {
        (Player::White.as_usize(), Player::Black.as_usize())
    } else {
        (Player::Black.as_usize(), Player::White.as_usize())
    };

    // Low material correction. Guard against an imaginary material advantage
    // that actually is a draw
    if pawn_material[stronger] == 0 {
        if piece_material[stronger] < PieceType::Rook.mg_value() {
            return 0;
        }

        if pawn_material[weaker] == 0
            && (piece_material[stronger] == 2 * PieceType::Knight.mg_value())
        {
            return 0;
        }

        if piece_material[stronger] == PieceType::Rook.mg_value()
            && (piece_material[weaker] == PieceType::Bishop.mg_value()
                || piece_material[weaker] == PieceType::Knight.mg_value())
        {
            score /= 2;
        }

        if (piece_material[stronger] == PieceType::Rook.mg_value() + PieceType::Bishop.mg_value()
            || piece_material[stronger]
                == PieceType::Rook.mg_value() + PieceType::Knight.mg_value())
            && piece_material[weaker] == PieceType::Rook.mg_value()
        {
            score /= 2;
        }
    }

    if board.turn == Player::White {
        score
    } else {
        -score
    }
}

#[inline(always)]
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
#[inline(always)]
fn mobility(board: &Board, piece: Piece, sq: Square, attacked_by: &mut AttackedBy) -> Score {
    if piece.t == PieceType::Pawn {
        return 0;
    }

    let occ = board.occ_bb();
    let my_bb = board.player_bb(piece.c);
    let opp_bb = occ & !my_bb;

    let moves = attacks(piece.t, sq, occ, piece.c);
    let att = moves & opp_bb;

    match piece.c {
        Player::White => attacked_by.white |= att,
        _ => attacked_by.black |= att,
    }

    let open = BitBoard::count(moves & !occ);
    let att = BitBoard::count(att);
    let def = BitBoard::count(moves & my_bb);

    // This score is in millipawns
    let score = (match piece.t {
        PieceType::Knight => 20 * open + 35 * att + 15 * def,
        PieceType::Bishop => 17 * open + 30 * att + 15 * def,
        PieceType::Rook => 15 * open + 20 * att + 15 * def,
        PieceType::Queen => 5 * open + 15 * att + 8 * def,
        PieceType::King => 4 * open + 15 * att + 10 * def,
        _ => panic!(),
    } / 30) as Score;

    match piece.c {
        Player::White => score,
        _ => -score,
    }
}

#[inline(always)]
const fn pawn_structure(side: Player, sq: Square, pawns: u64, opp_pawns: u64) -> Score {
    let mut score = 0;

    let file = sq % 8;
    // isolated pawn, as there are no pawns besides it
    if pawns & ISOLATED[file as usize] == 0 {
        score -= 8;
    }
    // doubled pawn
    if BitBoard::more_than_one(pawns & BitBoard::file_bb(sq)) {
        score -= 12;
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

#[inline(always)]
fn king_pawn_shield(
    board: &Board,
    w_pawns: u64,
    b_pawns: u64,
    mg: &mut [Score; 2],
    w_king_sq: Square,
    b_king_sq: Square,
    w_king_bb: u64,
    b_king_bb: u64,
) {
    // punish king on open or semi-open file
    if (w_pawns | b_pawns) & BitBoard::file_bb(w_king_sq) == 0 {
        mg[0] -= 13;
    } else if w_pawns & BitBoard::file_bb(w_king_sq) == 0 {
        mg[0] -= 5;
    }
    if (w_pawns | b_pawns) & BitBoard::file_bb(b_king_sq) == 0 {
        mg[1] -= 13;
    } else if b_pawns & BitBoard::file_bb(b_king_sq) == 0 {
        mg[1] -= 5;
    }

    let w_pawn_shield = SHIELDING_PAWNS[0][w_king_sq as usize];
    let w_king_front_span = ranks_in_front_of(Player::White, w_king_sq);
    mg[0] += missing_shield_pawns(w_pawn_shield, w_pawns, b_pawns, w_king_front_span);

    let b_pawn_shield = SHIELDING_PAWNS[1][b_king_sq as usize];
    let b_king_front_span = ranks_in_front_of(Player::Black, b_king_sq);
    mg[1] += missing_shield_pawns(b_pawn_shield, b_pawns, w_pawns, b_king_front_span);
}

/// # Arguments
///
/// * `king_front_span` - All the squares in front of the king
const fn missing_shield_pawns(
    mut pawn_shield: u64,
    pawns: u64,
    opp_pawns: u64,
    king_front_span: u64,
) -> i32 {
    let mut pawns_missing = 0;
    let mut pawns_open_file_missing = 0;
    while pawn_shield != 0 {
        let sq = BitBoard::bit_scan_forward(pawn_shield);
        let file_bb = BitBoard::file_bb(sq);
        if pawn_shield & pawns & file_bb == 0 {
            pawns_missing += 1;

            if opp_pawns & king_front_span & file_bb == 0 {
                pawns_open_file_missing += 1;
            }
        }

        pawn_shield &= !file_bb;
    }

    SHIELD_MISSING[pawns_missing] + SHIELD_MISSING_ON_OPEN_FILE[pawns_open_file_missing]
}

/// Reward the control of space on our side of the board
#[inline(always)]
const fn eval_space(
    board: &Board,
    side: Player,
    my_pawns: u64,
    attacked_by: &AttackedBy,
    non_pawn_material: Score,
) -> Score {
    // Space isn't important if there aren't pieces to control it, so return early
    if non_pawn_material < 11551 {
        return 0;
    }

    let opp = side.opp();

    let safe = SAFE_MASK[side.as_usize()] & !my_pawns & !attacked_by.pawns(opp);

    let mut behind = my_pawns;
    match side {
        Player::White => behind |= (behind >> 8) | (behind >> 16),
        _ => behind |= (behind << 8) | (behind << 16),
    }

    let bonus = BitBoard::count(safe) + BitBoard::count(behind & safe & !attacked_by.side(opp));
    // Increase space evaluation weight in positions with many minor pieces
    let weight =
        BitBoard::count(board.piece_bb(PieceType::Knight) | board.piece_bb(PieceType::Bishop));

    (bonus * weight * weight / 16) as Score
}

struct AttackedBy {
    pub white: u64,
    pub black: u64,
    pub w_pawns: u64,
    pub b_pawns: u64,
}

impl AttackedBy {
    pub const fn new() -> Self {
        AttackedBy {
            white: 0,
            black: 0,
            w_pawns: 0,
            b_pawns: 0,
        }
    }

    pub const fn side(&self, side: Player) -> u64 {
        match side {
            Player::White => self.white,
            _ => self.black,
        }
    }

    pub const fn pawns(&self, side: Player) -> u64 {
        match side {
            Player::White => self.w_pawns,
            _ => self.b_pawns,
        }
    }
}
