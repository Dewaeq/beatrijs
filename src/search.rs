use crate::bitmove::MoveFlag;
use crate::defs::{PieceType, Score, Square, INFINITY, MG_VALUE};
use crate::eval::evaluate;
use crate::search_info::SearchInfo;
use crate::table::{HashEntry, HashFlag, TWrapper};
use crate::utils::{is_draw, is_repetition, print_search_info};
use crate::{
    bitmove::BitMove, board::Board, defs::Player, movelist::MoveList, order::pick_next_move,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const MAX_SEARCH_DEPTH: usize = 100;
pub const IMMEDIATE_MATE_SCORE: Score = 31_000;
pub const IS_MATE: Score = IMMEDIATE_MATE_SCORE - 1000;

const DELTA_PRUNING: Score = 100;
const STATIC_NULL_MOVE_DEPTH: i32 = 5;
const STATIC_NULL_MOVE_MARGIN: Score = 120;


pub struct Searcher {
    pub num_nodes: u64,
    pub sel_depth: usize,
    pub board: Board,
    pub table: Arc<TWrapper>,
    abort: Arc<AtomicBool>,
    stop: bool,
    info: SearchInfo,
    best_root_move: u16,
    root_moves: MoveList,
    history_score: [[[Score; 64]; 64]; 2],
    quiets_tried: [[Option<u16>; 128]; MAX_SEARCH_DEPTH],
    eval_history: [Score; 128],
}

impl Searcher {
    pub fn new(board: Board, abort: Arc<AtomicBool>, tt: Arc<TWrapper>, info: SearchInfo) -> Self {
        Searcher {
            board,
            abort,
            stop: false,
            num_nodes: 0,
            sel_depth: 0,
            table: tt,
            info,
            best_root_move: 0,
            root_moves: MoveList::new(),
            history_score: [[[0; 64]; 64]; 2],
            quiets_tried: [[None; 128]; MAX_SEARCH_DEPTH],
            eval_history: [0; 128],
        }
    }

    fn start(&mut self) {
        self.info.start(self.board.turn);
        self.abort.store(false, Ordering::Relaxed);
    }

    fn stop(&mut self) {
        self.abort.store(true, Ordering::Relaxed);
        self.stop = true;
    }

    fn should_stop(&mut self) -> bool {
        if !self.stop {
            self.stop = self.abort.load(Ordering::SeqCst);
        }

        self.stop
    }

    fn checkup(&mut self) {
        if !self.info.has_time() {
            self.stop();
        }
    }

    fn clear_for_search(&mut self) {
        self.num_nodes = 0;
        self.board.pos.ply = 0;
        self.board.clear_killers();
    }

    pub fn iterate(&mut self) {
        self.start();
        self.clear_for_search();

        self.root_moves = MoveList::legal(&mut self.board);
        let mut score = -INFINITY;

        for depth in 1..=self.info.depth as i32 {
            score = self.aspiration_search(depth, score);

            if self.should_stop() {
                break;
            }

            let elapsed = self.info.started.elapsed().as_secs_f64() * 1000f64;
            let pv = self.table.extract_pv(&mut self.board, depth);

            self.best_root_move = pv[0];
            print_search_info(
                depth,
                self.sel_depth,
                score,
                elapsed,
                self.num_nodes,
                &pv,
                self.board.turn,
            );
        }

        let best_move = self
            .table
            .best_move(self.board.key())
            .unwrap_or(self.best_root_move);
        println!("bestmove {}", BitMove::pretty_move(best_move));
    }

    fn aspiration_search(&mut self, depth: i32, eval: Score) -> Score {
        let mut alpha = -INFINITY;
        let mut beta = INFINITY;

        if depth > 4 {
            alpha = eval - 16;
            beta = eval + 16;
        }

        let mut research = 0;
        loop {
            if self.should_stop() {
                return 0;
            }

            if alpha < -3500 {
                alpha = -INFINITY;
            }

            if beta > 3500 {
                beta = INFINITY;
            }

            let best_eval = self.negamax(depth, alpha, beta, false);
            research += 1;

            if best_eval <= alpha {
                alpha = (-INFINITY).max(alpha - research * research * 23);
            } else if best_eval >= beta {
                beta = INFINITY.min(beta + research * research * 23);
            } else {
                return best_eval;
            }
        }
    }

    fn negamax(
        &mut self,
        mut depth: i32,
        mut alpha: Score,
        mut beta: Score,
        do_null: bool,
    ) -> Score {
        if self.num_nodes & 4096 == 0 {
            self.checkup();
        }

        if self.should_stop() {
            return 0;
        }

        if is_draw(&self.board) {
            return 0;
        }

        let ply = self.board.pos.ply;
        if ply >= MAX_SEARCH_DEPTH {
            return evaluate(&self.board);
        }

        let is_root = ply == 0;
        let is_pv = beta - alpha > 1;
        let in_check = self.board.in_check();

        // Mate distance pruning
        if !is_root {
            alpha = Score::max(-IMMEDIATE_MATE_SCORE + ply as Score, alpha);
            beta = Score::min(IMMEDIATE_MATE_SCORE - 1 - ply as Score, beta);

            if alpha >= beta {
                return alpha;
            }
        }

        if in_check && !is_root {
            depth += 1;
        }

        if depth == 0 {
            let score = self.quiesence(alpha, beta, true, ply);
            return score;
        }

        let entry = self.table.probe(self.board.key(), ply);
        let mut tt_move = 0;
        let is_root = self.board.pos.ply == 0;

        if let Some(entry) = entry {
            tt_move = entry.m;

            if let Some(score) = table_cutoff(entry, depth, alpha, beta) {
                return score;
            }
        }

        self.num_nodes += 1;

        let mut moves = if is_root {
            self.root_moves
        } else {
            MoveList::legal(&mut self.board)
        };

        if moves.is_empty() {
            if in_check {
                return -IMMEDIATE_MATE_SCORE + ply as Score;
            }
            return 0;
        }

        let eval = if entry.is_some() {
            entry.unwrap().static_eval
        } else {
            evaluate(&self.board)
        };

        self.eval_history[ply] = eval;

        // Futility pruning: frontier node
        if depth == 1
            && !in_check
            && !is_pv
            && eval + MG_VALUE[1] < alpha
            && alpha > -IS_MATE
            && beta < IS_MATE
        {
            return eval;
        }

        /* if !in_check
            && !is_pv
            && depth < 9
            && eval - 165 * (depth as Score) >= beta
            && alpha > -IS_MATE
            && beta < IS_MATE
        {
            return eval;
        } */

        // Static null move pruning
        if depth <= STATIC_NULL_MOVE_DEPTH
            && !is_pv
            && !in_check
            && eval - STATIC_NULL_MOVE_MARGIN * depth >= beta
        {
            return eval;
        }

        // Null move pruning
        if do_null && !in_check && depth >= 4 && self.board.has_big_piece(self.board.turn) {
            self.board.make_null_move();
            let score = -self.negamax(depth - 4, -beta, -beta + 1, false);
            self.board.unmake_null_move();

            if self.should_stop() {
                return 0;
            }

            if score >= beta {
                return beta;
            }
        }

        let improving = (ply >= 2 && eval >= self.eval_history[ply - 2]);

        // Reverse futility pruning
        if !is_pv
            && !in_check
            && depth < 7
            && beta < IS_MATE
            && eval - 67 * depth + 76 * (improving as i32) >= beta
        {
            return eval;
        }

        // Razoring
        if !is_pv && !in_check && tt_move == 0 && do_null && depth <= 3 {
            let threshold = alpha - 300 - (depth - 1) * 60;
            if eval < threshold {
                let score = self.quiesence(alpha, beta, true, ply);
                // This might be a bit too bold, but it's worth a try
                return score;
                /* if score < threshold {
                    return alpha;
                } */
            }
        }
        
        let mut quiets_tried: usize = 0;
        let mut search_quiets = true;
        let mut best_move = 0;
        let mut best_score = -INFINITY;
        let old_alpha = alpha;

        if tt_move != 0 {
            set_tt_move_score(&mut moves, tt_move);
        }

        let turn = self.board.turn;

        for i in 0..moves.size() {
            pick_next_move(&mut moves, i);
            let (m, move_score) = moves.get_all(i);
            let is_cap = BitMove::is_cap(m);
            let is_prom = BitMove::is_prom(m);
            let is_quiet = !is_cap && !is_prom;
            let src = BitMove::src(m) as usize;
            let dest = BitMove::dest(m) as usize;
            let history_score = self.history_score[turn.as_usize()][src][dest];

            if !search_quiets && is_quiet {
                continue;
            }

            let gives_check = self.board.gives_check(m);
            let mut score = 0;

            if !is_root && best_score > -IS_MATE && self.board.has_non_pawns(turn) {
                if is_cap || is_prom || gives_check {
                    // History pruning: skip quiet moves at low depth
                    // that yielded bad results in previous searches
                    if depth <= 2 && history_score < 0 {
                        continue;
                    }

                    // SEE pruning
                    if !self.board.see_ge(m, -200 * depth) {
                        continue;
                    }

                    // Futility pruning
                    if depth <= 8 && move_score < -50 * depth * depth {
                        continue;
                    }
                } else {
                    // Futility pruning
                    if depth <= 3 && (eval + MG_VALUE[1] <= alpha) {
                        search_quiets = false;
                        continue;
                    }

                    // Late move pruning
                    if !in_check
                        && depth <= 4
                        && quiets_tried as u32 > (3 * 2u32.pow(depth as u32 - 1))
                    {
                        search_quiets = false;
                        continue;
                    }

                    // SEE pruning
                    if depth <= 8 && !self.board.see_ge(m, -21 * (depth * depth)) {
                        continue;
                    }
                }
            }

            let mut reduction = 0;
            if depth > 2 && (!is_cap || move_score < 0) && i > 1 && (!is_root || i > 4) {
                reduction = lmr_reduction(
                    depth,
                    i,
                    is_pv,
                    is_cap || is_prom,
                    improving,
                    gives_check,
                    in_check,
                    history_score,
                );
            }

            self.board.make_move(m);

            // search pv move in a full window, at full depth
            if i == 0 || depth <= 2 || !is_pv {
                score = -self.negamax(depth - 1 - reduction, -beta, -alpha, true);

                if reduction > 0 && score > alpha {
                    score = -self.negamax(depth - 1, -beta, -alpha, true);
                }
            } else {
                // Search every other move in a zero window
                score = -self.negamax(depth - 1 - reduction, -alpha - 1, -alpha, true);
                if score > alpha && score < beta {
                    score = -self.negamax(depth - 1, -beta, -alpha, true);
                }
            }

            self.board.unmake_move(m);

            if self.should_stop() {
                return 0;
            }

            if is_root {
                self.root_moves.set_score(i, score);
            }

            if score > alpha {
                alpha = score;
            }

            if score > best_score {
                best_score = score;
                best_move = m;

                if is_root {
                    self.best_root_move = m;
                }
            }

            if score >= beta {
                if !is_cap {
                    self.board.killers[1][ply] = self.board.killers[0][ply];
                    self.board.killers[0][ply] = m;

                    self.history_score[turn.as_usize()][src][dest] +=
                        depth as Score * depth as Score;

                    for i in 0..quiets_tried {
                        let mv = self.quiets_tried[ply][i].unwrap();
                        let m_src = BitMove::src(mv) as usize;
                        let m_dest = BitMove::dest(mv) as usize;
                        self.history_score[turn.as_usize()][m_src][m_dest] -=
                            depth as Score * depth as Score;
                    }
                }

                let entry = HashEntry::new(
                    self.board.pos.key,
                    depth,
                    best_move,
                    beta,
                    eval,
                    HashFlag::Beta,
                );
                self.table.store(entry, ply);

                return beta;
            } else if !is_cap {
                self.quiets_tried[ply][quiets_tried] = Some(m);
                quiets_tried += 1;
            }
        }

        if self.should_stop() {
            return 0;
        }

        let entry = if alpha != old_alpha {
            HashEntry::new(
                self.board.key(),
                depth,
                best_move,
                best_score,
                eval,
                HashFlag::Exact,
            )
        } else {
            HashEntry::new(
                self.board.key(),
                depth,
                best_move,
                alpha,
                eval,
                HashFlag::Alpha,
            )
        };

        self.table.store(entry, ply);

        alpha
    }

    fn quiesence(&mut self, mut alpha: Score, beta: Score, root: bool, depth: usize) -> Score {
        if self.num_nodes & 4096 == 0 {
            self.checkup();
        }

        if self.should_stop() {
            return 0;
        }

        if is_draw(&self.board) {
            return 0;
        }

        self.num_nodes += 1;
        if depth > self.sel_depth {
            self.sel_depth = depth;
        }

        // Stand pat
        let eval = evaluate(&self.board);
        if eval >= beta {
            return beta;
        }
        if eval > alpha {
            alpha = eval;
        }

        // delta pruning
        let diff = alpha - eval - DELTA_PRUNING;
        if diff > 0 && diff > max_gain(&self.board) {
            return eval;
        }

        let mut moves = MoveList::quiet(&mut self.board);

        for i in 0..moves.size() {
            pick_next_move(&mut moves, i);
            let m = moves.get(i);

            // This move (likely) won't raise alpha
            if !passes_delta(&self.board, m, eval, alpha) {
                continue;
            }

            if !self.board.see_ge(m, 0) {
                continue;
            }

            self.board.make_move(m);
            let score = -self.quiesence(-beta, -alpha, false, depth + 1);
            self.board.unmake_move(m);

            if score > alpha {
                alpha = score;
            }

            if score >= beta {
                return beta;
            }
        }

        if root {
            let entry = HashEntry::new(
                self.board.key(),
                0,
                self.table.best_move(self.board.key()).unwrap_or(0),
                alpha,
                eval,
                HashFlag::Exact,
            );
            self.table.store(entry, 0);
        }

        alpha
    }
}

#[inline(always)]
/// Biggest possible material gain in this position
const fn max_gain(board: &Board) -> Score {
    let mut score = 0;

    let opp = board.player_bb(board.turn.opp());
    if opp & board.piece_bb(PieceType::Queen) != 0 {
        score += MG_VALUE[PieceType::Queen.as_usize()];
    } else if opp & board.piece_bb(PieceType::Rook) != 0 {
        score += MG_VALUE[PieceType::Rook.as_usize()];
    } else if opp & board.piece_bb(PieceType::Bishop) != 0 {
        score += MG_VALUE[PieceType::Bishop.as_usize()];
    } else if opp & board.piece_bb(PieceType::Knight) != 0 {
        score += MG_VALUE[PieceType::Knight.as_usize()];
    }

    // Pawn about to promote
    if board.player_piece_bb(board.turn, PieceType::Pawn) & board.turn.rank_7() != 0 {
        score += MG_VALUE[PieceType::Queen.as_usize()] - MG_VALUE[PieceType::Pawn.as_usize()];
    }

    score
}

#[inline(always)]
/// Is this move eligible to increase alpha?
const fn passes_delta(board: &Board, m: u16, eval: Score, alpha: Score) -> bool {
    if eval >= alpha {
        return true;
    }

    if BitMove::is_prom(m) {
        return true;
    }

    let captured = match BitMove::flag(m) {
        MoveFlag::CAPTURE => board.piece_type(BitMove::dest(m)),
        MoveFlag::EN_PASSANT => PieceType::Pawn,
        /// if this move isn't a capture, then is must be a check, which we always want to search
        _ => return true,
    };

    eval + MG_VALUE[captured.as_usize()] + DELTA_PRUNING >= alpha
}

#[inline(always)]
fn set_tt_move_score(moves: &mut MoveList, tt_move: u16) {
    let mut i = 0;
    while i < moves.size() {
        if moves.get(i) == tt_move {
            moves.set_score(i, 2_000_000);
            break;
        }
        i += 1;
    }
}

const fn table_cutoff(entry: HashEntry, depth: i32, alpha: Score, beta: Score) -> Option<Score> {
    if entry.depth < depth as u8 {
        return None;
    }

    match entry.hash_flag {
        HashFlag::Exact => Some(entry.score),
        HashFlag::Alpha => {
            if alpha >= entry.score {
                Some(alpha)
            } else {
                None
            }
        }
        HashFlag::Beta => {
            if beta <= entry.score {
                Some(beta)
            } else {
                None
            }
        }
    }
}

fn lmr_reduction(
    depth: i32,
    index: usize,
    is_pv: bool,
    is_tactical: bool,
    improving: bool,
    gives_check: bool,
    in_check: bool,
    history_score: i32,
) -> i32 {
    let depth_ln = (depth as f32).ln();
    let index_ln = (index as f32).ln();
    let mut reduction = (0.8422840719846748 * index_ln * depth_ln
        - 0.4 * index_ln
        - 0.22572624883839026 * depth_ln
        + 1.2)
        .max(0f32);

    if is_tactical {
        reduction /= 2f32;
    }

    if is_pv {
        reduction = reduction * 0.66;
    }

    if !improving {
        reduction += 1f32;
    }

    if gives_check {
        reduction -= 1f32;
    }

    if in_check {
        reduction -= 2f32;
    }

    if history_score > 0 {
        reduction -= 1f32;
    }

    reduction = reduction.min(depth as f32 - 1f32);

    reduction.max(1f32) as i32
}
