use crate::{
    bitboard::BitBoard,
    board::Board,
    defs::{
        e, pieces::*, Eval, Piece, PieceType, Player, Score, Square, DARK_SQUARES, LIGHT_SQUARES, PHASE_MAX, SMALL_CENTER
    },
    gen::{
        attack::{attacks, knight_attacks, rook_attacks},
        tables::{CENTER_DISTANCE, DISTANCE, KING_ZONE, PASSED, SHIELDING_PAWNS},
    },
    movegen::{pawn_caps, pawn_push},
    params::*,
    utils::{east_one, file_fill, fill_down, fill_up, front_span, ranks_in_front_of, west_one},
};

pub const GAME_PHASE_INC: [Score; 6] = [0, 1, 1, 2, 4, 0];

const SAFE_MASK: [u64; 2] = [
    (BitBoard::FILE_C | BitBoard::FILE_D | BitBoard::FILE_E | BitBoard::FILE_F)
        & (BitBoard::RANK_2 | BitBoard::RANK_3 | BitBoard::RANK_4),
    (BitBoard::FILE_C | BitBoard::FILE_D | BitBoard::FILE_E | BitBoard::FILE_F)
        & (BitBoard::RANK_5 | BitBoard::RANK_6 | BitBoard::RANK_7),
];

#[derive(Default)]
pub struct EvalInfo {
    // Actual eval
    phase: Score,
    material: [Eval; 2],
    mob: [Eval; 2],
    tropism: [Eval; 2],
    king_shield: [Eval; 2],
    adjust_material: [Eval; 2],
    blockages: [Score; 2],
    positional_themes: [Score; 2],
    // Attack data
    pawns: [u64; 2],
    att_count: [Score; 2],
    att_weight: [Score; 2],
    attacked_by: [u64; 2],
    attack_by_pawns: [u64; 2],
    king_sq: [Square; 2],
    king_bb: [u64; 2],
}

impl EvalInfo {
    fn init(&mut self, board: &Board) {
        self.phase = board.pos.phase;
        self.material = [
            e!(board.pos.mg_score[0], board.pos.eg_score[0]),
            e!(board.pos.mg_score[1], board.pos.eg_score[1]),
        ];

        self.king_bb[0] = board.player_piece_bb(Player::White, PieceType::King);
        self.king_bb[1] = board.player_piece_bb(Player::Black, PieceType::King);
        self.king_sq[0] = BitBoard::bit_scan_forward(self.king_bb[0]);
        self.king_sq[1] = BitBoard::bit_scan_forward(self.king_bb[1]);

        self.pawns[0] = board.player_piece_bb(Player::White, PieceType::Pawn);
        self.pawns[1] = board.player_piece_bb(Player::Black, PieceType::Pawn);

        self.attack_by_pawns[0] = pawn_caps(self.pawns[0], Player::White);
        self.attack_by_pawns[1] = pawn_caps(self.pawns[1], Player::Black);
        self.attacked_by = self.attack_by_pawns;
    }
}

pub fn evaluate(board: &Board) -> Score {
    let mut eval = EvalInfo::default();
    eval.init(board);

    // Score is from white's perspective
    let mut score = pawn_score(board, &mut eval);

    let mut piece_bb = board.occ_bb() & !board.piece_bb(PieceType::Pawn);
    while piece_bb != 0 {
        let sq = BitBoard::pop_lsb(&mut piece_bb);
        let piece = board.piece(sq);

        score += mobility(board, piece, sq as Square, &mut eval);
    }

    mopup_eval(board, &mut eval);
    king_pawn_shield(board, &mut eval);
    adjust_material(board, &mut eval);

    score += eval_knights(board, Player::White, &eval) - eval_knights(board, Player::Black, &eval);
    score += eval_bishops(board, Player::White, &eval) - eval_bishops(board, Player::Black, &eval);
    score += eval_rooks(board, Player::White, &eval) - eval_rooks(board, Player::Black, &eval);

    score += eval.material[0] - eval.material[1];
    score += eval.king_shield[0] - eval.king_shield[1];
    score += eval.mob[0] - eval.mob[1];
    score += eval.tropism[0] - eval.tropism[1];
    score += eval.adjust_material[0] - eval.adjust_material[1];

    let mut total_score = score.phased(eval.phase.min(PHASE_MAX));

    // Tempo bonus
    if board.turn == Player::White {
        total_score += 10;
    } else {
        total_score -= 10;
    }

    // King safety:
    // Safety doesn't matter if we don't have enough pieces to actually attack
    if eval.att_count[0] < 2 || board.num_pieces(WHITE_QUEEN) == 0 {
        eval.att_weight[0] = 0;
    }

    if eval.att_count[1] < 2 || board.num_pieces(BLACK_QUEEN) == 0 {
        eval.att_weight[1] = 0;
    }

    total_score += SAFETY_TABLE[eval.att_weight[0] as usize];
    total_score -= SAFETY_TABLE[eval.att_weight[1] as usize];

    let piece_material = board.pos.piece_material;
    let total_non_pawn = piece_material[0] + piece_material[1];

    // Control of space on the player's side of the board
    total_score += eval_space(&board, Player::White, &eval, total_non_pawn);
    total_score -= eval_space(&board, Player::Black, &eval, total_non_pawn);

    let (stronger, weaker) = if total_score > 0 {
        (Player::White.as_usize(), Player::Black.as_usize())
    } else {
        (Player::Black.as_usize(), Player::White.as_usize())
    };

    // Low material correction. Guard against an imaginary material advantage
    // that actually is a draw
    if board.pos.num_pieces[stronger * 6] == 0 {
        if piece_material[stronger] < PieceType::Rook.mg_value() {
            return 0;
        }

        if board.pos.num_pieces[weaker * 6] == 0
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

#[inline(always)]
fn mopup_eval(board: &Board, eval: &mut EvalInfo) {
    // Don't apply mop-up when there are still pawns on the board
    if board.piece_bb(PieceType::Pawn) != 0 {
        return;
    }

    // Only apply mopup when we're up on material,
    // require at least a rook
    let us = board.turn.as_usize();
    let opp = 1 - us;
    let diff = eval.material[us].eg() - eval.material[opp].eg();
    if diff < PieceType::Rook.eg_value() - 100 {
        return;
    }

    let king_sq = eval.king_sq[us] as usize;
    let opp_king_sq = eval.king_sq[opp] as usize;

    let center_dist = 4.7 * CENTER_DISTANCE[opp_king_sq] as f32;
    let kings_dist = 1.6 * (14 - DISTANCE[king_sq][opp_king_sq]) as f32;
    let mopup = (center_dist + kings_dist) as Score;

    eval.mob[us] += e!(0, mopup);
}

fn pawn_score(board: &Board, eval: &mut EvalInfo) -> Eval {
    let w_score = eval_pawns(board, Player::White, eval);
    let b_score = eval_pawns(board, Player::Black, eval);

    w_score - b_score
}

fn adjust_material(board: &Board, eval: &mut EvalInfo) {
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
    //if board.num_pieces(WHITE_ROOK) > 1 {
    //eval.adjust_material[0] += ROOK_PAIR_PENALTY;
    //}
    //if board.num_pieces(BLACK_ROOK) > 1 {
    //eval.adjust_material[1] += ROOK_PAIR_PENALTY;
    //}

    eval.adjust_material[0] += KNIGHT_PAWN_ADJUSTMENT[board.num_pieces(WHITE_PAWN)]
        * (board.num_pieces(WHITE_KNIGHT) as Score);
    eval.adjust_material[1] += KNIGHT_PAWN_ADJUSTMENT[board.num_pieces(BLACK_PAWN)]
        * (board.num_pieces(BLACK_KNIGHT) as Score);
    eval.adjust_material[0] += ROOK_PAWN_ADJUSTMENT[board.num_pieces(WHITE_PAWN)]
        * (board.num_pieces(WHITE_ROOK) as Score);
    eval.adjust_material[1] += ROOK_PAWN_ADJUSTMENT[board.num_pieces(BLACK_PAWN)]
        * (board.num_pieces(BLACK_ROOK) as Score);
}

// Structural evaluation of a piece, from white's perspective
#[inline(always)]
fn mobility(board: &Board, piece: Piece, sq: Square, eval: &mut EvalInfo) -> Eval {
    let occ = board.occ_bb();
    let my_bb = board.player_bb(piece.c);
    let opp_bb = occ & !my_bb;
    let us = piece.c.as_usize();
    let opp = 1 - us;
    let opp_king_sq = eval.king_sq[opp];
    let opp_king_zone = KING_ZONE[opp][opp_king_sq as usize];

    let moves = attacks(piece.t, sq, occ, piece.c);
    let att = moves & opp_bb;
    let open = match piece.t {
        PieceType::Knight | PieceType::Bishop => moves & !occ & !eval.attack_by_pawns[opp],
        _ => moves & !occ,
    };

    match piece.c {
        Player::White => eval.attacked_by[0] |= att,
        _ => eval.attacked_by[1] |= att,
    }

    let open = BitBoard::count(open);
    let att = BitBoard::count(att);
    let def = BitBoard::count(moves & my_bb);
    let king_att_cnt = BitBoard::count(moves & !my_bb & opp_king_zone);

    // This score is in millipawns
    let score = (match piece.t {
        PieceType::Knight => 20 * open + 35 * att + 15 * def,
        PieceType::Bishop => 17 * open + 30 * att + 15 * def,
        PieceType::Rook => 15 * open + 20 * att + 15 * def,
        PieceType::Queen => 5 * open + 15 * att + 8 * def,
        PieceType::King => 2 * open + 8 * att + 10 * def,
        _ => panic!(),
    } / 10) as Score;

    let king_att_score = match piece.t {
        PieceType::Queen => 4 * king_att_cnt,
        PieceType::Rook => 3 * king_att_cnt,
        PieceType::Bishop | PieceType::Knight => 2 * king_att_cnt,
        _ => 0,
    };

    if king_att_score > 0 {
        eval.att_count[us] += 1;
        eval.att_weight[us] += king_att_score as Score;
    }

    match piece.c {
        Player::White => e!(score),
        _ => e!(-score),
    }
}

#[inline(always)]
fn king_pawn_shield(board: &Board, eval: &mut EvalInfo) {
    let [w_pawns, b_pawns] = eval.pawns;
    let [w_king_sq, b_king_sq] = eval.king_sq;

    // punish king on open or semi-open file
    if (w_pawns | b_pawns) & BitBoard::file_bb(w_king_sq) == 0 {
        eval.king_shield[0] += KING_OPEN;
    } else if w_pawns & BitBoard::file_bb(w_king_sq) == 0 {
        eval.king_shield[0] += KING_SEMI_OPEN;
    }
    if (w_pawns | b_pawns) & BitBoard::file_bb(b_king_sq) == 0 {
        eval.king_shield[1] += KING_OPEN;
    } else if b_pawns & BitBoard::file_bb(b_king_sq) == 0 {
        eval.king_shield[1] += KING_SEMI_OPEN;
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
fn missing_shield_pawns(
    mut pawn_shield: u64,
    pawns: u64,
    opp_pawns: u64,
    king_front_span: u64,
) -> Eval {
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
fn eval_space(board: &Board, side: Player, eval: &EvalInfo, non_pawn_material: Score) -> Score {
    // Space isn't important if there aren't pieces to control it, so return early
    if non_pawn_material < 11551 {
        return 0;
    }

    let us = side.as_usize();
    let opp = 1 - us;
    let my_pawns = eval.pawns[side.as_usize()];
    let safe = SAFE_MASK[side.as_usize()] & !my_pawns & !eval.attack_by_pawns[opp];

    let mut behind = my_pawns;
    match side {
        Player::White => behind |= (behind >> 8) | (behind >> 16),
        _ => behind |= (behind << 8) | (behind << 16),
    }

    let bonus = BitBoard::count(safe) + BitBoard::count(behind & safe & !eval.attack_by_pawns[opp]);
    // Increase space evaluation weight in positions with many minor pieces
    let weight = (board.num_pieces(WHITE_BISHOP)
        + board.num_pieces(BLACK_BISHOP)
        + board.num_pieces(WHITE_KNIGHT)
        + board.num_pieces(BLACK_KNIGHT)) as u32;

    (bonus * weight * weight / 16) as Score
}

fn eval_knights(board: &Board, side: Player, eval: &EvalInfo) -> Eval {
    let us = side.as_usize();
    let opp = 1 - us;
    let mut score = e!(0);

    let opp_pawns = eval.pawns[opp];
    let mut knights = board.player_piece_bb(side, PieceType::Knight);
    let mut supported = knights & eval.attack_by_pawns[side.as_usize()];

    while supported != 0 {
        let sq = BitBoard::pop_lsb(&mut supported);
        score += SUPPORTED_KNIGHT;
        // Check if this is an outpost knight, i.e. it can't be attacked by a pawn on the neighbouring files
        if PASSED[side.as_usize()][sq as usize] & opp_pawns & !BitBoard::file_bb(sq) == 0 {
            score += OUTPOST_KNIGHT;
        }
    }

    let mut connected = 0;
    while knights != 0 {
        let sq = BitBoard::pop_lsb(&mut knights);
        let moves = knight_attacks(sq);
        connected += BitBoard::count(moves & knights);
    }

    score += CONNECTED_KNIGHT * connected as Score;

    score
}

fn eval_bishops(board: &Board, side: Player, eval: &EvalInfo) -> Eval {
    let us = side.as_usize();
    let opp = 1 - us;
    let my_pawns = eval.pawns[us];
    let opp_pawns = eval.pawns[opp];
    let mut score = e!(0);

    let bishops = board.player_piece_bb(side, PieceType::Bishop);

    if bishops & DARK_SQUARES != 0 {
        score += BISHOP_PAWN_COLOR * BitBoard::count(my_pawns & DARK_SQUARES) as Score;
        score += BISHOP_OPP_PAWN_COLOR * BitBoard::count(opp_pawns & DARK_SQUARES) as Score;
    }
    if bishops & LIGHT_SQUARES != 0 {
        score += BISHOP_PAWN_COLOR * BitBoard::count(my_pawns & LIGHT_SQUARES) as Score;
        score += BISHOP_OPP_PAWN_COLOR * BitBoard::count(opp_pawns & LIGHT_SQUARES) as Score;
    }

    score
}

fn eval_rooks(board: &Board, side: Player, eval: &EvalInfo) -> Eval {
    let us = side.as_usize();
    let opp = 1 - us;

    let mut score = e!(0);

    let opp_king_bb = eval.king_bb[opp];
    let opp_king_file = BitBoard::file_bb(eval.king_sq[opp]);
    let occ = board.occ_bb();
    let opp_pawns = eval.pawns[opp];
    let mut rooks = board.player_piece_bb(side, PieceType::Rook);

    // Rooks on seventh rank are only valuable if they cut of the king
    // or can goble up some pawns
    if opp_king_bb & side.rank_8() != 0 || opp_pawns & side.rank_7() != 0 {
        score += ROOK_ON_SEVENTH * BitBoard::count(rooks & side.rank_7()) as Score;
    }

    // Align an attack on enemy king
    score += ROOK_KING_ALIGNED * BitBoard::count(rooks & opp_king_file) as Score;

    // Connected rooks
    let mut connected = 0;
    while BitBoard::several(rooks) {
        let sq = BitBoard::pop_lsb(&mut rooks);
        let moves = rook_attacks(sq, occ);
        connected += BitBoard::count(moves & rooks);
    }

    score += CONNECTED_ROOK * connected as Score;

    score
}

fn eval_pawns(board: &Board, side: Player, eval: &EvalInfo) -> Eval {
    let mut score = e!(0);
    let occ = board.occ_bb();
    let us = side.as_usize();
    let opp = 1 - us;

    let my_pawns = eval.pawns[us];
    let opp_pawns = eval.pawns[opp];
    let my_pawn_attacks = eval.attack_by_pawns[us];
    let opp_pawn_attacks = eval.attack_by_pawns[opp];

    // Defended pawns
    let supported = my_pawns & my_pawn_attacks;
    score += PAWN_DEFENDED * BitBoard::count(supported) as Score;

    // Pawns controlling centre of the board
    let num_pawns_behind_center =
        BitBoard::count(my_pawns & pawn_caps(SMALL_CENTER, side.opp())) as Score;
    score += PAWN_BEHIND_CENTER * num_pawns_behind_center;

    // Pawn mobility
    let attacks = pawn_caps(my_pawns & !side.rank_7(), side);
    let pushes = pawn_push(my_pawns, side) & !occ;
    let double_pushes = pawn_push(pushes & side.rank_3(), side);

    score += PAWN_ATTACK * BitBoard::count(attacks) as Score;
    score += PAWN_PUSH * BitBoard::count(pushes) as Score;
    score += PAWN_DOUBLE_PUSH * BitBoard::count(double_pushes) as Score;

    // Doubled and isolated pawns
    let my_front_span = front_span(side, my_pawns);
    let num_doubled = BitBoard::count(my_pawns & my_front_span) as Score;
    let num_isolated =
        BitBoard::count(file_fill(my_pawns) & !west_one(my_pawns) & !east_one(my_pawns)) as Score;

    score += DOUBLED_PAWN * num_doubled;
    score += ISOLATED_PAWN * num_isolated;

    // Backward pawns, see https://www.chessprogramming.org/Backward_Pawns_(Bitboards)#Telestop_Weakness
    let my_attack_spans = fill_up(side, my_pawn_attacks);
    let stops = !my_attack_spans & opp_pawn_attacks;
    let my_backward_area = fill_down(side, stops);
    let num_backward = BitBoard::count(my_backward_area & my_pawns) as Score;

    score += BACKWARD_PAWN * num_backward;

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

    score += ROOK_BEHIND_PASSER * num_my_rooks_behind_passers;
    score += OPP_ROOK_BEHIND_PASSER * num_opp_rooks_behind_passers;

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
