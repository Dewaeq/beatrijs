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
const STATIC_NULL_MOVE_DEPTH: i32 = 5;
const STATIC_NULL_MOVE_MARGIN: Score = 120;

#[derive(Clone, Copy, Debug)]
pub struct SearchInfo {
    pub depth: i32,
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
    pub fn depth(depth: i32) -> Self {
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
    history_score: [[[Score; 64]; 64]; 2],
    quiets_tried: [[Option<u16>; 128]; 128],
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
            history_score: [[[0; 64]; 64]; 2],
            quiets_tried: [[None; 128]; 128],
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

        for depth in 1..=self.info.depth as i32 {
            let mut score = self.search(depth, alpha, beta);

            if self.should_stop() {
                break;
            }

            // score is outside of the window, so do a full-width search
            if score <= alpha || score >= beta {
                alpha = -INFINITY;
                beta = INFINITY;
                score = self.search(depth, alpha, beta);
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

    fn search(&mut self, depth: i32, alpha: Score, beta: Score) -> Score {
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
        mut depth: i32,
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

        if depth >= MAX_DEPTH {
            return evaluate(&self.board);
        }

        let ply = self.board.pos.ply;
        let is_root = ply == 0;

        // Mate distance pruning
        if !is_root {
            alpha = Score::max(-IMMEDIATE_MATE_SCORE + ply as Score, alpha);
            beta = Score::min(IMMEDIATE_MATE_SCORE - 1 - ply as Score, beta);

            if alpha >= beta {
                return alpha;
            }
        }

        if depth == 0 {
            let score = self.quiesence(alpha, beta, true);
            return score;
        }

        let entry = self.table.probe(self.board.key(), ply);
        let in_check = self.board.in_check();
        let mut tt_move = 0;
        let mut is_pv = false;

        if let Some(entry) = entry {
            tt_move = entry.m;
            is_pv = true;

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

        // Razoring
        if !is_pv && !in_check && tt_move == 0 && do_null && depth <= 3 {
            let threshold = alpha - 300 - (depth - 1) * 60;
            if eval < threshold {
                let score = self.quiesence(alpha, beta, true);
                // This might be a bit too bold, but it's worth a try
                return score;
                /* if score < threshold {
                    return alpha;
                } */
            }
        }

        if in_check && !is_root {
            depth += 1;
        }

        let mut quiets_tried: usize = 0;
        let mut search_quiets = true;
        let mut best_move = 0;
        let mut best_score = -INFINITY;
        let old_alpha = alpha;

        if tt_move != 0 {
            set_tt_move_score(&mut moves, tt_move);
        }

        let is_prunable = !is_root && !in_check && !is_pv && (alpha > -IS_MATE && beta < IS_MATE);
        let can_prune = is_prunable && depth <= 3 && (eval + MG_VALUE[1] <= alpha);

        let turn = self.board.turn;

        for i in 0..moves.size() {
            pick_next_move(&mut moves, i);
            let (m, move_score) = moves.get_all(i);
            let is_cap = BitMove::is_cap(m);
            let is_prom = BitMove::is_prom(m);
            let is_quiet = !is_cap && !is_prom;
            let src = BitMove::src(m) as usize;
            let dest = BitMove::dest(m) as usize;

            if !search_quiets && is_quiet {
                continue;
            }

            self.board.make_move(m);
            let gives_check = self.board.in_check();
            let mut score = 0;

            if !is_root
                && is_quiet
                && best_score > -IS_MATE
                && self.board.has_big_piece(turn)
                && !gives_check
            {
                // History pruning: skip quiet moves at low depth
                // that yielded bad results in previous searches
                if depth <= 2 && self.history_score[turn.as_usize()][src][dest] < 0 {
                    self.board.unmake_move(m);
                    continue;
                }

                // Futility pruning
                if can_prune {
                    self.board.unmake_move(m);
                    search_quiets = false;
                    continue;
                }

                if !in_check && depth <= 4 && quiets_tried as u32 > (3 * 2u32.pow(depth as u32 - 1))
                {
                    self.board.unmake_move(m);
                    search_quiets = false;
                    continue;
                }
            } else if !is_root
                && !gives_check
                && is_cap
                && best_score > -IS_MATE
                && depth <= 8
                && move_score < -50 * depth * depth
                && self.board.has_non_pawns(turn)
            {
                self.board.unmake_move(m);
                continue;
            }

            // search pv move in a full window, at full depth
            if i == 0 {
                score = -self.negamax(depth - 1, -beta, -alpha, true);
            } else {
                score = alpha + 1;
                // LMR
                // Dot not reduce moves that give check, capture (except bad captures) or promote
                if depth >= 3
                    && (!BitMove::is_tactical(m) || move_score < 0)
                    // && !gives_check
                    // && !in_check
                    && i > 3
                    && moves.size() > 20
                    // && m != self.board.killers[0][ply-1]
                    // && m != self.board.killers[1][ply-1]
                {
                    let mut r = 2;
                    if is_cap {
                        r = 1;
                    }
                    if beta - alpha > 1 {
                        r += 1;
                    }
                    if gives_check || in_check {
                        r = 1;
                    }

                    score = -self.negamax(depth - 1 - r, -alpha - 1, -alpha, true);
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
                    NodeType::Beta,
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
                NodeType::Exact,
            )
        } else {
            HashEntry::new(
                self.board.key(),
                depth,
                best_move,
                alpha,
                eval,
                NodeType::Alpha,
            )
        };

        self.table.store(entry, ply);

        alpha
    }

    fn quiesence(&mut self, mut alpha: Score, beta: Score, root: bool) -> Score {
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
            let score = -self.quiesence(-beta, -alpha, false);
            self.board.unmake_move(m);

            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }

        if root {
            let entry = HashEntry::new(
                self.board.key(),
                0,
                self.table.best_move(self.board.key()).unwrap_or(0),
                alpha,
                eval,
                NodeType::Exact,
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
        MoveFlag::CAPTURE => board.piece(BitMove::dest(m)),
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

    match entry.node_type {
        NodeType::Exact => Some(entry.score),
        NodeType::Alpha => {
            if alpha >= entry.score {
                Some(alpha)
            } else {
                None
            }
        }
        NodeType::Beta => {
            if beta <= entry.score {
                Some(beta)
            } else {
                None
            }
        }
    }
}
