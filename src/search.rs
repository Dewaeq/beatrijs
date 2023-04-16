use crate::bitmove::MoveFlag;
use crate::defs::{PieceType, Score, INFINITY, MAX_DEPTH, MG_VALUE};
use crate::eval::evaluate;
use crate::table::{HashEntry, NodeType, TWrapper};
use crate::utils::{is_draw, is_repetition, print_search_info};
use crate::{
    bitmove::BitMove, board::Board, defs::Player, movelist::MoveList, order::pick_next_move,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub const IMMEDIATE_MATE_SCORE: Score = 31_000;
pub const IS_MATE: Score = IMMEDIATE_MATE_SCORE - 1000;

const DELTA_PRUNING: Score = 100;
const STATIC_NULL_MOVE_DEPTH: u8 = 5;
const STATIC_NULL_MOVE_MARGIN: Score = 120;

#[derive(Clone, Copy, Debug)]
pub struct SearchInfo {
    pub depth: u8,
    pub w_time: usize,
    pub b_time: usize,
    pub w_inc: usize,
    pub b_inc: usize,
    pub move_time: usize,
    pub started: Instant,
}

impl Default for SearchInfo {
    fn default() -> Self {
        Self {
            depth: MAX_DEPTH,
            w_time: 0,
            b_time: 0,
            w_inc: 0,
            b_inc: 0,
            move_time: 0,
            started: Instant::now(),
        }
    }
}

impl SearchInfo {
    pub fn depth(depth: u8) -> Self {
        let mut info = SearchInfo::default();
        info.depth = depth;
        info
    }

    pub fn my_time(&self, side: Player) -> usize {
        match side {
            Player::White => self.w_time,
            Player::Black => self.b_time,
        }
    }

    pub fn start(&mut self) {
        self.started = Instant::now();
    }

    pub fn has_time(&self, side: Player) -> bool {
        (self.started.elapsed().as_millis() as usize + 50) < self.my_time(side) / 30
    }
}

pub struct Searcher {
    pub num_nodes: u64,
    pub board: Board,
    pub table: Arc<TWrapper>,
    abort: Arc<AtomicBool>,
    start_time: Instant,
    info: SearchInfo,
    best_root_move: u16,
    root_moves: MoveList,
}

impl Searcher {
    pub fn new(board: Board, abort: Arc<AtomicBool>, tt: Arc<TWrapper>, info: SearchInfo) -> Self {
        Searcher {
            board,
            abort,
            num_nodes: 0,
            table: tt,
            start_time: Instant::now(),
            info,
            best_root_move: 0,
            root_moves: MoveList::new(),
        }
    }

    fn start(&mut self) {
        self.start_time = Instant::now();
        self.abort.store(false, Ordering::Relaxed);
    }

    fn stop(&mut self) {
        self.abort.store(true, Ordering::Relaxed);
    }

    fn should_stop(&self) -> bool {
        self.abort.load(Ordering::SeqCst)
    }

    fn clear_for_search(&mut self) {
        self.num_nodes = 0;
        self.board.pos.ply = 0;
        self.board.clear_killers();
    }

    pub fn iterate(&mut self) {
        self.start();

        // save alpha and beta for aspiration search
        let mut alpha = -INFINITY;
        let mut beta = INFINITY;

        self.root_moves = MoveList::legal(&mut self.board);

        for depth in 1..=self.info.depth {
            let mut score = self.search(depth as u8, alpha, beta);

            if self.should_stop() {
                break;
            }

            // score is outside of the window, so do a full-width search
            if score <= alpha || score >= beta {
                alpha = -INFINITY;
                beta = INFINITY;
                score = self.search(depth as u8, alpha, beta);
            }

            if score.abs() > IS_MATE || is_repetition(&self.board) {
                break;
            }

            // aspiration search:
            // slightly shrink the search window
            alpha = score - 50;
            beta = score + 50;

            self.num_nodes = 0;
        }

        let best_move = self
            .table
            .best_move(self.board.key())
            .unwrap_or(self.best_root_move);
        println!("bestmove {}", BitMove::pretty_move(best_move));
    }

    fn search(&mut self, depth: u8, alpha: Score, beta: Score) -> Score {
        self.clear_for_search();

        let start = Instant::now();
        let score = self.negamax(depth, alpha, beta, false);
        let elapsed = self.start_time.elapsed();
        let total_time = (elapsed.as_secs_f64() * 1000f64) as u64;
        let search_time = start.elapsed().as_secs_f64();

        if !self.should_stop() {
            let pv = self.table.extract_pv(&mut self.board, depth);
            self.best_root_move = pv[0];
            print_search_info(
                depth,
                score,
                total_time,
                search_time,
                self.num_nodes,
                &pv,
                self.board.turn,
            );
        }

        score
    }

    fn negamax(
        &mut self,
        mut depth: u8,
        mut alpha: Score,
        mut beta: Score,
        do_null: bool,
    ) -> Score {
        if self.should_stop() {
            return 0;
        }

        if is_draw(&self.board) {
            return 0;
        }

        if depth == 0 {
            let score = self.quiesence(alpha, beta);

            let node_type = if score >= beta {
                NodeType::Beta
            } else if score > alpha {
                NodeType::Alpha
            } else {
                NodeType::Exact
            };

            let entry = HashEntry::new(self.board.key(), depth, 0, score, node_type);
            self.table.store(entry, self.board.pos.ply);

            return score;
        }

        let entry = self.table.probe(self.board.key(), self.board.pos.ply);
        let in_check = self.board.in_check();
        let mut pv_move = 0;
        let mut is_pv = false;
        let is_root = self.board.pos.ply == 0;

        if let Some(entry) = entry {
            pv_move = entry.m;
            is_pv = true;

            if entry.depth >= depth {
                match entry.node_type {
                    NodeType::Exact => return entry.score,
                    NodeType::Alpha => {
                        if alpha >= entry.score {
                            return alpha;
                        }
                    }
                    NodeType::Beta => {
                        if beta <= entry.score {
                            return beta;
                        }
                    }
                }
            }
        }

        self.num_nodes += 1;

        if self.board.pos.ply > 0 {
            alpha = Score::max(-IMMEDIATE_MATE_SCORE + self.board.pos.ply as Score, alpha);
            beta = Score::min(IMMEDIATE_MATE_SCORE - self.board.pos.ply as Score, beta);

            if alpha >= beta {
                return alpha;
            }
        }

        let mut moves = if is_root {
            self.root_moves
        } else {
            MoveList::legal(&mut self.board)
        };

        if moves.is_empty() {
            if in_check {
                return -IMMEDIATE_MATE_SCORE + self.board.pos.ply as Score;
            }
            return 0;
        }

        let eval = evaluate(&self.board);

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
            && eval - STATIC_NULL_MOVE_MARGIN * (depth as Score) >= beta
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

        if in_check && !is_root {
            depth += 1;
        }

        let mut best_move = 0;
        let mut best_score = -INFINITY;
        let old_alpha = alpha;

        if pv_move != 0 {
            let mut i = 0;
            while i < moves.size() {
                if moves.get(i) == pv_move {
                    moves.set_score(i, 2_000_000);
                    break;
                }
                i += 1;
            }
        }

        let is_prunable = !in_check && !is_pv && (alpha > -IS_MATE && beta < IS_MATE);
        let can_prune = is_prunable && depth <= 3 && (eval + MG_VALUE[1] <= alpha);

        for i in 0..moves.size() {
            pick_next_move(&mut moves, i);
            let m = moves.get(i);

            self.board.make_move(m);
            let gives_check = self.board.in_check();
            let mut score = 0;

            // Futility pruning
            if can_prune && !gives_check {
                self.board.unmake_move(m);
                continue;
            }

            // search pv move in a full window, at full depth
            if i == 0 {
                score = -self.negamax(depth - 1, -beta, -alpha, true);
            } else {
                score = alpha + 1;
                // LMR
                // Dot not reduce moves that give check, capture or promote
                if depth >= 3
                    && !BitMove::is_tactical(m)
                    && !gives_check
                    && !in_check
                    && i > 3
                    && moves.size() > 20
                    && m != self.board.killers[0][self.board.pos.ply - 1]
                    && m != self.board.killers[1][self.board.pos.ply - 1]
                {
                    let mut d = depth - 3;
                    d = u8::min(d, d - (i / 20) as u8);
                    if is_pv {
                        d = u8::min(d + 2, depth - 1);
                    }
                    score = -self.negamax(d, -alpha - 1, -alpha, true);
                }

                if score > alpha {
                    score = -self.negamax(depth - 1, -alpha - 1, -alpha, true);
                    if score > alpha && score < beta {
                        score = -self.negamax(depth - 1, -beta, -alpha, true);
                    }
                }
            }

            self.board.unmake_move(m);

            if self.should_stop() {
                return 0;
            }

            if is_root {
                self.root_moves.set_score(i, score);
            }

            if score > best_score {
                best_score = score;
                best_move = m;

                if is_root {
                    self.best_root_move = m;
                }

                if score > alpha {
                    if score >= beta {
                        if !BitMove::is_cap(m) {
                            let ply = self.board.pos.ply;
                            self.board.killers[1][ply] = self.board.killers[0][ply];
                            self.board.killers[0][ply] = m;
                        }

                        let entry = HashEntry::new(
                            self.board.pos.key,
                            depth,
                            best_move,
                            beta,
                            NodeType::Beta,
                        );

                        self.table.store(entry, self.board.pos.ply);
                        return beta;
                    }

                    alpha = score;
                }
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
                NodeType::Exact,
            )
        } else {
            HashEntry::new(self.board.key(), depth, best_move, alpha, NodeType::Alpha)
        };

        self.table.store(entry, self.board.pos.ply);

        alpha
    }

    fn quiesence(&mut self, mut alpha: Score, beta: Score) -> Score {
        if is_draw(&self.board) {
            return 0;
        }

        self.num_nodes += 1;

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

            self.board.make_move(m);
            let score = -self.quiesence(-beta, -alpha);
            self.board.unmake_move(m);

            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
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
        MoveFlag::CAPTURE => board.piece(BitMove::dest(m)),
        MoveFlag::EN_PASSANT => PieceType::Pawn,
        /// if this move isn't a capture, then is must be a check, which we always want to search
        _ => return true,
    };

    eval + MG_VALUE[captured.as_usize()] + DELTA_PRUNING >= alpha
}
