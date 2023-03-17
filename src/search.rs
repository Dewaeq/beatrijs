use crate::defs::{Score, INFINITY};
use crate::eval::evaluate;
use crate::table::{HashEntry, NodeType, TWrapper};
use crate::utils::print_search_info;
use crate::{
    bitboard::BitBoard,
    bitmove::BitMove,
    board::Board,
    defs::{Piece, Player, Value},
    movelist::MoveList,
    order::pick_next_move,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub const IMMEDIATE_MATE_SCORE: Score = 30_000;
pub const IS_MATE: Score = IMMEDIATE_MATE_SCORE - 64;

pub struct Searcher {
    pub num_nodes: u64,
    pub board: Board,
    pub table: Arc<TWrapper>,
    abort: Arc<AtomicBool>,
}

impl Default for Searcher {
    fn default() -> Self {
        Searcher::new(
            Board::start_pos(),
            Arc::new(AtomicBool::new(false)),
            Arc::new(TWrapper::new()),
        )
    }
}

impl Searcher {
    pub fn new(board: Board, abort: Arc<AtomicBool>, tt: Arc<TWrapper>) -> Self {
        Searcher {
            board,
            abort,
            num_nodes: 0,
            table: tt,
        }
    }

    pub fn start(&mut self) {
        self.abort.store(false, Ordering::Relaxed);
    }

    pub fn stop(&mut self) {
        self.abort.store(true, Ordering::Relaxed);
    }

    fn should_stop(&self) -> bool {
        self.abort.load(Ordering::SeqCst)
    }

    pub fn iterate(&mut self, max_depth: u8) {
        self.start();

        // save alpha and beta for aspiration search
        let mut alpha = -INFINITY;
        let mut beta = INFINITY;

        for depth in 1..=max_depth {
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

            // aspiration search:
            // slightly shrink the search window
            alpha = score - 50;
            beta = score + 50;

            self.num_nodes = 0;
        }
    }

    pub fn search(&mut self, depth: u8, alpha: Score, beta: Score) -> Score {
        self.num_nodes = 0;
        let start = Instant::now();
        let score = self.negamax(depth, 0, alpha, beta, false);
        let end = start.elapsed();
        let time = (end.as_secs_f64() * 1000f64) as u64;

        if !self.should_stop() {
            let best_move = self.table.best_move(self.board.key());
            print_search_info(depth, score, time, best_move.unwrap_or(0), self.num_nodes);
        }

        score
    }

    fn negamax(
        &mut self,
        mut depth: u8,
        ply_from_root: u8,
        mut alpha: Score,
        mut beta: Score,
        do_null: bool,
    ) -> Score {
        if self.should_stop() {
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
            self.table.store(entry, ply_from_root);

            return score;
        }

        let entry = self.table.probe(self.board.key());
        let mut pv_move = 0;

        if let Some(entry) = entry {
            pv_move = entry.m;

            if entry.depth >= depth {
                let mut score = entry.score;
                if score > IS_MATE {
                    score -= ply_from_root as Score;
                } else if score < -IS_MATE {
                    score += ply_from_root as Score;
                }

                match entry.node_type {
                    NodeType::Exact => return entry.score,
                    NodeType::Alpha => {
                        if alpha >= score {
                            return alpha;
                        }
                    }
                    NodeType::Beta => {
                        if beta <= score {
                            return beta;
                        }
                    }
                }
            }
        }

        self.num_nodes += 1;

        if ply_from_root > 0 {
            alpha = Score::max(-IMMEDIATE_MATE_SCORE + ply_from_root as Score, alpha);
            beta = Score::min(IMMEDIATE_MATE_SCORE - ply_from_root as Score, beta);

            if alpha >= beta {
                return alpha;
            }
        }

        let mut moves = MoveList::legal(&mut self.board);
        if moves.is_empty() {
            if self.board.in_check() {
                return -IMMEDIATE_MATE_SCORE + ply_from_root as Score;
            }
            return 0;
        }

        // TODO: add zugzwang protection
        if do_null && !self.board.in_check() && depth >= 4 {
            self.board.make_null_move();
            let score = -self.negamax(depth - 4, ply_from_root + 1, -beta, -beta + 1, false);
            self.board.unmake_null_move();

            if self.should_stop() {
                return 0;
            }

            if score >= beta {
                return beta;
            }
        }

        if self.board.in_check() {
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

        for i in 0..moves.size() {
            pick_next_move(&mut moves, i);
            let m = moves.get(i);

            self.board.make_move(m);
            let score = -self.negamax(depth - 1, ply_from_root + 1, -beta, -alpha, true);
            self.board.unmake_move(m);

            if self.should_stop() {
                return 0;
            }

            if score > best_score {
                best_score = score;
                best_move = m;

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

                        self.table.store(entry, ply_from_root);
                        return beta;
                    }

                    alpha = score;
                }
            }
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

        self.table.store(entry, ply_from_root);

        alpha
    }

    fn quiesence(&mut self, mut alpha: Score, beta: Score) -> Score {
        self.num_nodes += 1;

        let stand_pat = evaluate(&self.board);
        if stand_pat >= beta {
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        let mut moves = MoveList::quiet(&mut self.board);

        for i in 0..moves.size() {
            pick_next_move(&mut moves, i);
            let m = moves.get(i);

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
