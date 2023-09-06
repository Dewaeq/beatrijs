use crate::bitmove::MoveFlag;
use crate::defs::{PieceType, Score, MG_VALUE};
use crate::eval::evaluate;
use crate::gen::tables::LMR;
use crate::movegen::{is_legal_move, MovegenParams};
use crate::search_info::SearchInfo;
use crate::table::{Bound, HashEntry, TWrapper};
use crate::utils::{is_draw, print_search_info};
use crate::{bitmove::BitMove, board::Board, movelist::MoveList, order::pick_next_move};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const INFINITY: Score = 32_000;
pub const MAX_SEARCH_DEPTH: usize = 100;
pub const MATE: Score = 31_000;
pub const IS_MATE: Score = MATE - 1000;

pub type HistoryTable = [[[Score; 64]; 64]; 2];

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
    history_score: HistoryTable,
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
            self.stop = self.abort.load(Ordering::Relaxed);
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

        let params = MovegenParams::new(&self.board, &self.history_score, 0);
        self.root_moves = MoveList::all(params);

        let mut score = -INFINITY;

        for depth in 1..=self.info.depth as i32 {
            score = self.aspiration_search(depth, score);

            if self.should_stop() {
                break;
            }

            let elapsed = self.info.started.elapsed().as_secs_f64() * 1000f64;
            let pv = self.table.extract_pv(&mut self.board, depth);
            // let hash_full = self.table.hash_full();

            if pv.len() > 0 {
                self.best_root_move = pv[0];
            }
            print_search_info(
                depth,
                self.sel_depth,
                score,
                elapsed,
                self.num_nodes,
                0,
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

    fn aspiration_search(&mut self, search_depth: i32, score: Score) -> Score {
        let mut alpha = -INFINITY;
        let mut beta = INFINITY;
        let mut delta = 12;
        let mut depth = search_depth;

        if depth > 4 {
            alpha = (-INFINITY).max(score - delta);
            beta = INFINITY.min(score + delta);
        }

        let mut research = 0;
        loop {
            if self.should_stop() {
                return 0;
            }

            let best_score = self.negamax(depth.max(1), alpha, beta, false);

            if (best_score <= alpha) {
                beta = (alpha + beta) / 2;
                alpha = (-INFINITY).max(alpha - delta);
                depth = search_depth;
            } else if best_score >= beta {
                beta = INFINITY.min(beta + delta);
                depth -= (best_score.abs() <= IS_MATE) as Score;
            } else {
                return best_score;
            }

            delta += delta / 2;
        }
    }

    fn negamax(
        &mut self,
        mut depth: i32,
        mut alpha: Score,
        mut beta: Score,
        do_null: bool,
    ) -> Score {
        assert!(alpha < beta);

        if self.num_nodes & 4096 == 0 {
            self.checkup();
        }

        if self.should_stop() {
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
            alpha = Score::max(-MATE + ply as Score, alpha);
            beta = Score::min(MATE - 1 - ply as Score, beta);

            if alpha >= beta {
                return alpha;
            }

            if is_draw(&self.board) {
                return 8 - (self.num_nodes & 7) as Score;
            }
        }

        if in_check && !is_root {
            depth += 1;
        }

        if depth <= 0 && !in_check {
            let score = self.quiescence(alpha, beta);
            return score;
        }

        let (tt_hit, entry) = self.table.probe(self.board.key(), ply);
        let mut tt_move = 0;
        let is_root = self.board.pos.ply == 0;

        if tt_hit {
            tt_move = entry.m;

            if !is_pv || entry.bound == Bound::Exact {
                if let Some(score) = table_cutoff(entry, depth, alpha, beta) {
                    return score;
                }

                if will_fail_low(entry, depth, alpha) {
                    return alpha;
                }
            }
        }

        self.num_nodes += 1;

        let mut moves = if is_root {
            let params = MovegenParams::new(&self.board, &self.history_score, tt_move);
            MoveList::all(params)
        } else {
            let params = MovegenParams::new(&self.board, &self.history_score, tt_move);
            MoveList::all(params)
        };

        if moves.is_empty() {
            if self.board.pos.ply > self.sel_depth {
                self.sel_depth = self.board.pos.ply;
            }

            if in_check {
                return -MATE + ply as Score;
            }
            return 0;
        }

        let static_eval = if in_check {
            -INFINITY
        } else if tt_hit {
            entry.static_eval
        } else {
            evaluate(&self.board)
        };

        if !tt_hit && !in_check {
            self.table.store_eval(self.board.key(), static_eval);
        }

        self.eval_history[ply] = static_eval;

        // Static null move pruning (= reverse futility pruning)
        /* if depth <= STATIC_NULL_MOVE_DEPTH
            && !is_pv
            && !in_check
            && eval - STATIC_NULL_MOVE_MARGIN * depth >= beta
        {
            return eval;
        }
        */

        // Null move pruning:
        // We have such a good position, that, even with a free move for our opponent,
        // we'll still be able to beat beta
        if do_null
            && !is_pv
            && !in_check
            && depth >= 2
            && static_eval >= beta
            && (!tt_hit || entry.bound == Bound::Lower || entry.score >= beta)
            && self.board.has_non_pawns(self.board.turn)
        {
            self.board.make_null_move();
            let r = 4 + depth / 6 + ((static_eval - beta) / 200).min(3);
            let score = -self.negamax((depth - r).max(0), -beta, -beta + 1, false);
            self.board.unmake_null_move();

            if score >= beta {
                if score > IS_MATE {
                    return beta;
                }
                return score;
            }
        }

        let improving = (!in_check && ply >= 2 && static_eval >= self.eval_history[ply - 2]);

        // Reverse futility pruning
        if !is_pv
            && !in_check
            && depth < 9
            && static_eval - 214 * (depth - improving as i32) >= beta
            && static_eval < 10_000
        {
            return static_eval;
        }

        // Futility pruning: frontier node
        if depth == 1
            && !in_check
            && !is_pv
            && static_eval + MG_VALUE[3] < alpha
            && alpha > -IS_MATE
            && beta < IS_MATE
        {
            return static_eval;
        }

        // Razoring
        if !is_pv && !in_check && tt_move == 0 && do_null && depth <= 3 {
            if static_eval + 300 + (depth - 1) * 60 < alpha {
                return self.quiescence(alpha, beta);
            }
        }

        let mut legals = 0;
        let mut quiets_tried: usize = 0;
        let mut search_quiets = true;
        let mut best_move = 0;
        let mut best_score = -INFINITY;
        let old_alpha = alpha;

        if tt_move != 0 {
            set_tt_move_score(&mut moves, tt_move);
        }

        let turn = self.board.turn;

        // Internal Iterative Reduction (IRR):
        // A modern approach to internal iterative deepening
        if depth >= 4 && (!tt_hit || tt_move == 0) {
            depth -= 1;
        }

        for i in 0..moves.size() {
            pick_next_move(&mut moves, i);
            let (m, move_score) = moves.get_all(i);

            if !is_legal_move(&self.board, m) {
                continue;
            }

            legals += 1;

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

            if !is_root && best_score > -IS_MATE && self.board.has_non_pawns(turn) {
                if is_cap || is_prom || gives_check {
                    // History pruning: skip quiet moves at low depth
                    // that yielded bad results in previous searches
                    if depth <= 2 && history_score < 0 && !gives_check {
                        continue;
                    }

                    // SEE pruning
                    if !self.board.see_ge(m, -200 * depth) {
                        continue;
                    }

                    // Futility pruning
                    if depth <= 8 && move_score < -50 * depth * depth && !gives_check {
                        continue;
                    }
                } else {
                    // Futility pruning: parent node
                    if !in_check && depth <= 8 && (static_eval + MG_VALUE[1] + 30 * depth <= alpha)
                    {
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
                    if depth <= 8 && !self.board.see_ge(m, -21 * depth * depth) {
                        continue;
                    }
                }
            }

            let mut reduction = 0;
            if depth > 2 && (!is_cap || move_score < 0) && legals > 1 && (!is_root || legals > 4) {
                reduction = lmr_reduction(
                    depth,
                    legals,
                    is_pv,
                    is_cap || is_prom,
                    improving,
                    gives_check,
                    in_check,
                    history_score,
                );
            }

            self.board.make_move(m);
            let mut score = 0;

            // search pv move in a full window, at full depth
            if legals == 0 || depth <= 2 || !is_pv {
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

            if is_root {
                self.root_moves.set_score(i, score);
            }

            if score > alpha {
                alpha = score;
            }

            if score > best_score && !self.should_stop() {
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

                    self.history_score[turn.as_usize()][src][dest] += depth * depth;

                    for i in 0..quiets_tried {
                        let mv = self.quiets_tried[ply][i].unwrap();
                        let m_src = BitMove::src(mv) as usize;
                        let m_dest = BitMove::dest(mv) as usize;
                        self.history_score[turn.as_usize()][m_src][m_dest] -= depth * depth;
                    }
                }

                break;
            } else if !is_cap {
                self.quiets_tried[ply][quiets_tried] = Some(m);
                quiets_tried += 1;
            }
        }

        if legals == 0 {
            if in_check {
                best_score = -MATE + self.board.pos.ply as Score;
            } else {
                best_score = 0;
            }
        }

        if !self.should_stop() {
            let entry = HashEntry::new(
                self.board.key(),
                depth,
                best_move,
                best_score,
                static_eval,
                if best_score >= beta {
                    Bound::Lower
                } else if alpha != old_alpha {
                    Bound::Exact
                } else {
                    Bound::Upper
                },
            );

            self.table.store(entry, ply);
        }

        best_score
    }

    fn quiescence(&mut self, mut alpha: Score, beta: Score) -> Score {
        if self.num_nodes & 4096 == 0 {
            self.checkup();
        }

        if self.should_stop() {
            return 0;
        }

        if is_draw(&self.board) {
            return 8 - (self.num_nodes & 7) as Score;
        }

        let mut tt_move = 0;

        let (tt_hit, entry) = self.table.probe(self.board.key(), self.board.pos.ply);
        if tt_hit {
            if let Some(score) = table_cutoff(entry, 0, alpha, beta) {
                return score;
            }

            tt_move = entry.m;
        }

        self.num_nodes += 1;
        if self.board.pos.ply > self.sel_depth {
            self.sel_depth = self.board.pos.ply;
        }

        let in_check = self.board.in_check();

        // Stand pat
        let eval = if tt_hit && entry.static_eval != -INFINITY {
            entry.static_eval
        } else {
            evaluate(&self.board)
        };

        if !tt_hit && !in_check {
            self.table.store_eval(self.board.key(), eval);
        }

        if eval >= beta {
            return eval;
        }
        if eval > alpha {
            alpha = eval;
        }

        // delta pruning
        let diff = alpha - eval - DELTA_PRUNING;
        if diff > 0 && diff > max_gain(&self.board) {
            return eval;
        }

        let params = MovegenParams::new(&self.board, &self.history_score, tt_move);
        let mut moves = MoveList::quiet(params);

        let mut legals = 0;
        let mut best_score = eval;
        let mut best_move = 0;
        let old_alpha = alpha;

        let futility_base = if self.board.in_check() {
            -INFINITY
        } else {
            eval + 155
        };

        if tt_move != 0 {
            set_tt_move_score(&mut moves, tt_move);
        }

        for i in 0..moves.size() {
            pick_next_move(&mut moves, i);
            let m = moves.get(i);

            if !is_legal_move(&self.board, m) {
                continue;
            }

            let is_prom = BitMove::is_prom(m);
            let gives_check = self.board.gives_check(m);

            legals += 1;

            // Futility pruning
            if !gives_check && futility_base > -INFINITY && !is_prom {
                if legals > 2 {
                    continue;
                }

                let dest = BitMove::dest(m);
                // We can safely do this, as this move isn't a promotion and it doesn't give check,
                // so it must be a capture
                let futility_value = futility_base + self.board.piece_type(dest).eg_value();

                if futility_value <= alpha {
                    best_score = best_score.max(futility_value);
                    continue;
                }

                if futility_base <= alpha && !self.board.see_ge(m, 1) {
                    best_score = best_score.max(futility_base);
                    continue;
                }
            }

            // This move (likely) won't raise alpha
            if !passes_delta(&self.board, m, eval, alpha) {
                continue;
            }

            // if eval + SEE exceeds beta, return early, as the opponent should've
            // had a better option earlier
            let see = self.board.see_approximate(m);
            if see + eval > beta {
                best_score = see;
                break;
            }

            if !self.board.see_ge(m, 0) {
                continue;
            }

            self.board.make_move(m);
            let score = -self.quiescence(-beta, -alpha);
            self.board.unmake_move(m);

            if score > best_score {
                best_score = score;
                best_move = m;
            }

            if score > alpha {
                alpha = score;
            }

            if score >= beta {
                break;
            }
        }

        if !self.should_stop() {
            let entry = HashEntry::new(
                self.board.key(),
                0,
                best_move,
                best_score,
                eval,
                if best_score >= beta {
                    Bound::Lower
                } else if alpha != old_alpha {
                    Bound::Exact
                } else {
                    Bound::Upper
                },
            );

            self.table.store(entry, self.board.pos.ply);
        }

        best_score
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
            moves.set_score(i, 8_000_000);
            break;
        }
        i += 1;
    }
}

const fn table_cutoff(entry: HashEntry, depth: i32, alpha: Score, beta: Score) -> Option<Score> {
    if entry.depth < depth as u8 {
        return None;
    }

    match entry.bound {
        Bound::None => None,
        Bound::Exact => Some(entry.score),
        Bound::Upper => {
            if alpha >= entry.score {
                Some(alpha)
            } else {
                None
            }
        }
        Bound::Lower => {
            if beta <= entry.score {
                Some(beta)
            } else {
                None
            }
        }
    }
}

/// If the entry from one depth lower failed low and, even with an added margin, it
/// still can't beat the current alpha, it will likely fail low again, so return early
fn will_fail_low(entry: HashEntry, depth: i32, alpha: Score) -> bool {
    entry.depth as i32 >= depth - 1
        && entry.bound == Bound::Upper
        && entry.score + MG_VALUE[0] <= alpha
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
    let mut reduction = LMR[depth.min(31) as usize][index.min(63)];

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
