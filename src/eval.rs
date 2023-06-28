use crate::{
    bitboard::BitBoard,
    board::Board,
    defs::{
        Piece, PieceType, Player, Score, Square, BLACK_BISHOP, BLACK_KNIGHT, BLACK_PAWN,
        BLACK_ROOK, CASTLE_KING_FILES, CASTLE_QUEEN_FILES, CENTER_SQUARES, DARK_SQUARES, EG_VALUE,
        LIGHT_SQUARES, MG_VALUE, PASSED_PAWN_SCORE, SMALL_CENTER, WHITE_BISHOP, WHITE_KNIGHT,
        WHITE_PAWN, WHITE_ROOK,
    },
    gen::{
        attack::{attacks, king_attacks},
        pesto::{EG_TABLE, MG_TABLE},
        tables::{CENTER_DISTANCE, DISTANCE, ISOLATED, PASSED, SHIELDING_PAWNS},
    },
    movegen::{pawn_caps, pawn_push},
    utils::{east_one, file_fill, fill_down, fill_up, front_span, ranks_in_front_of, west_one},
};

const GAME_PHASE_INC: [Score; 6] = [0, 1, 1, 2, 4, 0];
const BISHOP_PAIR_BONUS: Score = 23;
const KNIGHT_PAIR_PENALTY: Score = -8;
const ROOK_PAIR_PENALTY: Score = -22;
const KNIGHT_PAWN_ADJUSTMENT: [Score; 9] = [-20, -16, -12, -8, -4, 0, 4, 8, 12];
const ROOK_PAWN_ADJUSTMENT: [Score; 9] = [15, 12, 9, 6, 3, 0, -3, -6, -9];

const SHIELD_MISSING: [i32; 4] = [-2, -23, -38, -55];
const SHIELD_MISSING_ON_OPEN_FILE: [i32; 4] = [-8, -10, -37, -66];

const SAFE_MASK: [u64; 2] = [
    (BitBoard::FILE_C | BitBoard::FILE_D | BitBoard::FILE_E | BitBoard::FILE_F)
        & (BitBoard::RANK_2 | BitBoard::RANK_3 | BitBoard::RANK_4),
    (BitBoard::FILE_C | BitBoard::FILE_D | BitBoard::FILE_E | BitBoard::FILE_F)
        & (BitBoard::RANK_5 | BitBoard::RANK_6 | BitBoard::RANK_7),
];

pub struct Evaluation {
    phase: i32,
    mg_mob: [Score; 2],
    eg_mob: [Score; 2],
    mg_tropism: [Score; 2],
    eg_tropism: [Score; 2],
    att_count: [Score; 2],
    att_weight: [Score; 2],
    king_shield: [Score; 2],
    adjust_material: [Score; 2],
    blockages: [Score; 2],
    positional_themes: [Score; 2],
}

impl Evaluation {
    const fn new() -> Self {
        Evaluation {
            phase: 0,
            mg_mob: [0; 2],
            eg_mob: [0; 2],
            mg_tropism: [0; 2],
            eg_tropism: [0; 2],
            att_count: [0; 2],
            att_weight: [0; 2],
            king_shield: [0; 2],
            adjust_material: [0; 2],
            blockages: [0; 2],
            positional_themes: [0; 2],
        }
    }
}

pub fn evaluate(board: &Board) -> Score {
    let mut eval = Evaluation::new();

    let mut mg_material = [0; 2];
    let mut eg_material = [0; 2];
    let mut piece_material = [0; 2];
    let mut pawn_material = [0; 2];

    let mut sq = 0;
    for piece in board.pieces {
        if piece.is_none() {
            sq += 1;
            continue;
        }

        let idx = piece.c.as_usize();
        let pc_idx = piece.as_usize();
        mg_material[idx] += MG_TABLE[pc_idx][sq];
        eg_material[idx] += EG_TABLE[pc_idx][sq];

        if piece.t != PieceType::Pawn {
            piece_material[idx] += piece.t.mg_value();
        } else {
            pawn_material[idx] += piece.t.mg_value();
        }

        eval.phase += GAME_PHASE_INC[piece.t.as_usize()];

        sq += 1;
    }

    let mut total_score = 0;
    let mut mg_score = mg_material[0] - mg_material[1];
    let mut eg_score = eg_material[0] - eg_material[1];

    king_pawn_shield(board, &mut eval);
    mg_score += (eval.king_shield[0] - eval.king_shield[1]);

    // Tempo bonus
    if board.turn == Player::White {
        total_score += 10;
    } else {
        total_score -= 10;
    }

    adjust_material(board, &mut eval);
    total_score += pawn_score(board);

    /*
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

        if piece.t == PieceType::Pawn {
            pawn_material[idx] += MG_VALUE[0];
        } else {
            score += mobility(board, piece, sq as Square, &mut attacked_by);
            piece_material[idx] += MG_VALUE[piece.t.as_usize()];
        }

        sq += 1;
    }

    mopup_eval(board, &mut eg);

    // undeveloped pieces penalty
    let w_bishops = board.player_piece_bb(Player::White, PieceType::Bishop);
    let b_bishops = board.player_piece_bb(Player::Black, PieceType::Bishop);
    let w_knights = board.player_piece_bb(Player::White, PieceType::Knight);
    let b_knights = board.player_piece_bb(Player::Black, PieceType::Knight);
    mg[0] -= (BitBoard::count((w_knights | w_bishops) & BitBoard::RANK_1) * 6) as Score;
    mg[1] -= (BitBoard::count((b_knights | b_bishops) & BitBoard::RANK_8) * 6) as Score;

    // pawn attacks
    let w_pawn_attacks = pawn_caps(w_pawns, Player::White);
    let b_pawn_attacks = pawn_caps(b_pawns, Player::Black);

    attacked_by.w_pawns = w_pawn_attacks;
    attacked_by.white |= w_pawn_attacks;
    attacked_by.b_pawns = b_pawn_attacks;
    attacked_by.black |= b_pawn_attacks;

    score += eval_knights(board, Player::White, w_pawn_attacks, b_pawns);
    score -= eval_knights(board, Player::Black, b_pawn_attacks, w_pawns);

    score += eval_bishops(board, Player::White, w_pawns);
    score -= eval_bishops(board, Player::Black, b_pawns);

    // attacks on king
    let w_king_sq = board.king_square(Player::White);
    let b_king_sq = board.king_square(Player::Black);

    let w_king_bb = BitBoard::from_sq(w_king_sq);
    let b_king_bb = BitBoard::from_sq(b_king_sq);

    score -= (BitBoard::count(attacked_by.black & king_attacks(w_king_sq)) * 9) as Score;
    score -= (BitBoard::count(attacked_by.black & w_king_bb) * 16) as Score;

    score += (BitBoard::count(attacked_by.white & king_attacks(b_king_sq)) * 9) as Score;
    score += (BitBoard::count(attacked_by.white & b_king_bb) * 16) as Score;

    // Control of space on the player's side of the board
    let total_non_pawn = piece_material[0] + piece_material[1];
    score += eval_space(&board, Player::White, w_pawns, &attacked_by, total_non_pawn);
    score -= eval_space(&board, Player::Black, b_pawns, &attacked_by, total_non_pawn);
    */
    let mg_weight = eval.phase;
    let eg_weight = 24 - eval.phase;

    total_score += (mg_score * mg_weight + eg_score * eg_weight) / 24;

    let (stronger, weaker) = if total_score > 0 {
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
            total_score /= 2;
        }

        if (piece_material[stronger] == PieceType::Rook.mg_value() + PieceType::Bishop.mg_value()
            || piece_material[stronger]
                == PieceType::Rook.mg_value() + PieceType::Knight.mg_value())
            && piece_material[weaker] == PieceType::Rook.mg_value()
        {
            total_score /= 2;
        }
    }

    if board.turn == Player::White {
        total_score
    } else {
        -total_score
    }
}

fn pawn_score(board: &Board) -> Score {
    let w_score = eval_pawns(board, Player::White, 0, 0, 0, 0);
    let b_score = 0;

    w_score - b_score
}

fn adjust_material(board: &Board, eval: &mut Evaluation) {
    if board.num_pieces(WHITE_BISHOP) > 1 {
        eval.adjust_material[0] += BISHOP_PAIR_BONUS;
    }
    if board.num_pieces(BLACK_BISHOP) > 1 {
        eval.adjust_material[1] += BISHOP_PAIR_BONUS;
    }
    if board.num_pieces(WHITE_KNIGHT) > 1 {
        eval.adjust_material[0] += KNIGHT_PAIR_PENALTY;
    }
    if board.num_pieces(BLACK_KNIGHT) > 1 {
        eval.adjust_material[1] += KNIGHT_PAIR_PENALTY;
    }
    if board.num_pieces(WHITE_ROOK) > 1 {
        eval.adjust_material[0] += ROOK_PAIR_PENALTY;
    }
    if board.num_pieces(BLACK_ROOK) > 1 {
        eval.adjust_material[1] += ROOK_PAIR_PENALTY;
    }

    eval.adjust_material[0] += KNIGHT_PAWN_ADJUSTMENT[board.num_pieces(WHITE_PAWN)]
        * (board.num_pieces(WHITE_KNIGHT) as Score);
    eval.adjust_material[1] += KNIGHT_PAWN_ADJUSTMENT[board.num_pieces(BLACK_PAWN)]
        * (board.num_pieces(BLACK_KNIGHT) as Score);
    eval.adjust_material[0] += ROOK_PAWN_ADJUSTMENT[board.num_pieces(WHITE_PAWN)]
        * (board.num_pieces(WHITE_ROOK) as Score);
    eval.adjust_material[0] += ROOK_PAWN_ADJUSTMENT[board.num_pieces(BLACK_PAWN)]
        * (board.num_pieces(BLACK_ROOK) as Score);
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
fn king_pawn_shield(board: &Board, eval: &mut Evaluation) {
    let w_pawns = board.player_piece_bb(Player::White, PieceType::Pawn);
    let b_pawns = board.player_piece_bb(Player::Black, PieceType::Pawn);

    let w_king_sq = board.king_square(Player::White);
    let b_king_sq = board.king_square(Player::Black);

    // punish king on open or semi-open file
    if (w_pawns | b_pawns) & BitBoard::file_bb(w_king_sq) == 0 {
        eval.king_shield[0] -= 13;
    } else if w_pawns & BitBoard::file_bb(w_king_sq) == 0 {
        eval.king_shield[0] -= 5;
    }
    if (w_pawns | b_pawns) & BitBoard::file_bb(b_king_sq) == 0 {
        eval.king_shield[1] -= 13;
    } else if b_pawns & BitBoard::file_bb(b_king_sq) == 0 {
        eval.king_shield[1] -= 5;
    }

    let w_pawn_shield = SHIELDING_PAWNS[0][w_king_sq as usize];
    let w_king_front_span = ranks_in_front_of(Player::White, w_king_sq);
    eval.king_shield[0] += missing_shield_pawns(w_pawn_shield, w_pawns, b_pawns, w_king_front_span);

    let b_pawn_shield = SHIELDING_PAWNS[1][b_king_sq as usize];
    let b_king_front_span = ranks_in_front_of(Player::Black, b_king_sq);
    eval.king_shield[1] += missing_shield_pawns(b_pawn_shield, b_pawns, w_pawns, b_king_front_span);
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

fn eval_knights(board: &Board, side: Player, my_pawn_attacks: u64, opp_pawns: u64) -> Score {
    let mut score = 0;

    let knights = board.player_piece_bb(side, PieceType::Knight);
    let mut supported = knights & my_pawn_attacks;

    while supported != 0 {
        let sq = BitBoard::pop_lsb(&mut supported);
        // Check if this is an outpost knight, i.e. it can't be attacked by a pawn on the neighbouring files
        if PASSED[side.as_usize()][sq as usize] & opp_pawns & !BitBoard::file_bb(sq) == 0 {
            score += 25;
        }
    }

    score
}

fn eval_bishops(board: &Board, side: Player, my_pawns: u64) -> Score {
    let mut score = 0;

    let mut bishops = board.player_piece_bb(side, PieceType::Bishop);
    if BitBoard::more_than_one(bishops) {
        score += BISHOP_PAIR_BONUS;
    }

    if bishops & DARK_SQUARES != 0 {
        score -= BitBoard::count(my_pawns & DARK_SQUARES) as Score;
    }
    if bishops & LIGHT_SQUARES != 0 {
        score -= BitBoard::count(my_pawns & LIGHT_SQUARES) as Score;
    }

    score
}

fn eval_pawns(
    board: &Board,
    side: Player,
    my_pawns: u64,
    opp_pawns: u64,
    my_pawn_attacks: u64,
    opp_pawn_attacks: u64,
) -> Score {
    let mut score = 0;
    let occ = board.occ_bb();

    // Defended pawns
    let mut supported = my_pawns & my_pawn_attacks;
    while supported != 0 {
        let sq = BitBoard::pop_lsb(&mut supported);
        score += 5;
    }

    // Pawns controlling centre of the board
    let num_pawns_behind_center =
        BitBoard::count(my_pawns & pawn_caps(SMALL_CENTER, side.opp())) as Score;
    score += num_pawns_behind_center * -20;

    // Pawn mobility
    let attacks = pawn_caps(my_pawns & !side.rank_7(), side);
    let pushes = pawn_push(my_pawns, side) & !occ;
    let double_pushes = pawn_push(pushes & side.rank_3(), side);

    score += (BitBoard::count(attacks) * 7) as Score;
    score += (BitBoard::count(pushes) * 4) as Score;
    score += (BitBoard::count(double_pushes) * 3) as Score;

    // Doubled and isolated pawns
    let my_front_span = front_span(side, my_pawns);
    let num_doubled = BitBoard::count(my_pawns & my_front_span) as Score;
    let num_isolated =
        BitBoard::count(file_fill(my_pawns) & !west_one(my_pawns) & !east_one(my_pawns)) as Score;

    score += num_doubled * -11;
    score += num_isolated * -8;

    // Backward pawns, see https://www.chessprogramming.org/Backward_Pawns_(Bitboards)#Telestop_Weakness
    let my_attack_spans = fill_up(side, my_pawn_attacks);
    let stops = !my_attack_spans & opp_pawn_attacks;
    let my_backward_area = fill_down(side, stops);
    let num_backward = BitBoard::count(my_backward_area & my_pawns) as Score;

    score += num_backward * -6;

    // Passed pawns
    let mut opp_front_spans = front_span(side.opp(), opp_pawns);
    opp_front_spans |= west_one(opp_front_spans) | east_one(opp_front_spans);
    let mut passers = my_pawns & !opp_front_spans;
    let behind_passers = fill_down(side, passers);
    let num_my_rooks_behind_passers =
        BitBoard::count(board.player_piece_bb(side, PieceType::Rook) & behind_passers) as Score;
    let num_opp_rooks_behind_passers =
        BitBoard::count(board.player_piece_bb(side.opp(), PieceType::Rook) & behind_passers)
            as Score;

    score += num_my_rooks_behind_passers * 7;
    score += num_opp_rooks_behind_passers * -13;

    while passers != 0 {
        let sq = BitBoard::pop_lsb(&mut passers);
        let rel_rank = match side {
            Player::White => (sq / 8) as usize,
            Player::Black => (7 - sq / 8) as usize,
        };
        score += PASSED_PAWN_SCORE[rel_rank];
    }

    score
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
