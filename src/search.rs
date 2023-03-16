use crate::defs::MAX_MOVES;
use crate::table::{HashEntry, HashTable, NodeType, TWrapper, TABLE_SIZE, TT};
use crate::utils::print_search_info;
use crate::{
    bitboard::BitBoard,
    bitmove::BitMove,
    board::Board,
    defs::{Piece, Player, Value},
    movelist::MoveList,
    order::pick_next_move,
};
use std::cell::UnsafeCell;
use std::cmp;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

const IMMEDIATE_MATE_SCORE: i32 = 100000;

pub struct Searcher {
    pub num_nodes: u64,
    pub board: Board,
    pub table: Arc<TWrapper>,
    // table: Arc<TWrapper>,
    // abort: Arc<AtomicBool>,
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
            num_nodes: 0,
            board,
            table: tt,
        }
    }

    pub fn start(&mut self) {
        // self.abort.store(false, Ordering::Relaxed);
    }

    pub fn stop(&mut self) {
        // self.abort.store(true, Ordering::Relaxed);
    }

    fn should_stop(&self) -> bool {
        // self.num_nodes & 2047 != 0 && self.abort.load(Ordering::Relaxed)
        false
    }

    pub fn iterate(&mut self, max_depth: u8) {
        self.start();

        // save alpha and beta for aspiration search
        let mut alpha = i32::MIN + 1;
        let mut beta = i32::MAX - 1;

        for depth in 1..max_depth {
            let mut score = self.search(depth as u8, alpha, beta);

            if self.should_stop() {
                break;
            }

            // score is outside of the window, so do a full-width search
            if score <= alpha || score >= beta {
                alpha = i32::MIN + 1;
                beta = i32::MAX - 1;
                score = self.search(depth as u8, alpha, beta);
            }

            // aspiration search:
            // slightly shrink the search window
            alpha = score - 50;
            beta = score + 50;

            self.num_nodes = 0;
        }
    }

    pub fn search(&mut self, depth: u8, alpha: i32, beta: i32) -> i32 {
        self.num_nodes = 0;
        let start = Instant::now();
        let score = self.negamax(depth, 0, alpha, beta, false);
        let end = start.elapsed();
        let time = (end.as_secs_f64() * 1000f64) as u64;

        if !self.should_stop() {
            let best_move = unsafe { (*self.table.inner.get()).best_move(self.board.pos.key) };
            // let best_move = self.table.best_move(self.board.pos.key);
            print_search_info(depth, score, time, best_move.unwrap(), self.num_nodes);
        }

        score
    }

    fn negamax(
        &mut self,
        mut depth: u8,
        ply_from_root: u8,
        mut alpha: i32,
        mut beta: i32,
        do_null: bool,
    ) -> i32 {
        if self.should_stop() {
            return 0;
        }

        let entry = unsafe { (*self.table.inner.get()).probe(self.board.pos.key, depth) };
        // let entry = self.table.probe(self.board.pos.key, depth);

        let mut best_move = 0;
        if let Some(entry) = entry {
            if entry.depth >= depth {
                match entry.node_type {
                    NodeType::Exact => return entry.score,
                    NodeType::Alpha => {
                        if alpha >= entry.score {
                            return entry.score;
                        }
                    }
                    NodeType::Beta => {
                        if beta <= entry.score {
                            return entry.score;
                        }
                    }
                }
            }

            best_move = entry.m;
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

            let entry = HashEntry::new(self.board.pos.key, depth, 0, score, node_type);
            // self.table.store(entry);

            unsafe {
                (*self.table.inner.get()).store(entry);
            }

            return score;
        }

        self.num_nodes += 1;
        let old_alpha = alpha;

        if ply_from_root > 0 {
            alpha = i32::max(-IMMEDIATE_MATE_SCORE + ply_from_root as i32, alpha);
            beta = i32::min(IMMEDIATE_MATE_SCORE - ply_from_root as i32, beta);

            if alpha >= beta {
                return alpha;
            }
        }

        let mut moves = MoveList::legal(&mut self.board);
        if moves.is_empty() {
            if self.board.in_check() {
                return -IMMEDIATE_MATE_SCORE + ply_from_root as i32;
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

        if best_move != 0 {
            let mut i = 0;
            while i < moves.size() {
                if moves.get(i) == best_move {
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

            if score >= beta {
                if !BitMove::is_cap(m) {
                    let ply = self.board.pos.ply;
                    self.board.killers[1][ply] = self.board.killers[0][ply];
                    self.board.killers[0][ply] = m;
                }

                let entry =
                    HashEntry::new(self.board.pos.key, depth, best_move, score, NodeType::Beta);
                // self.table.store(entry);

                unsafe {
                    (*self.table.inner.get()).store(entry);
                }

                return beta;
            }
            if score > alpha {
                alpha = score;
                best_move = m;
            }
        }

        let node_type = if alpha > old_alpha {
            NodeType::Exact
        } else {
            NodeType::Alpha
        };

        let entry = HashEntry::new(self.board.pos.key, depth, best_move, alpha, node_type);
        // self.table.store(entry);
        unsafe {
            (*self.table.inner.get()).store(entry);
        }

        alpha
    }

    fn quiesence(&mut self, mut alpha: i32, beta: i32) -> i32 {
        if self.should_stop() {
            return 0;
        }

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

pub fn evaluate(board: &Board) -> i32 {
    let white_material = count_material(board, Player::White);
    let black_material = count_material(board, Player::Black);

    let eval = if board.turn == Player::White {
        white_material - black_material
    } else {
        black_material - white_material
    };

    eval
}

fn count_material(board: &Board, side: Player) -> i32 {
    let mut score = 0;

    score += BitBoard::count(board.player_piece_bb(side, Piece::Pawn)) as i32 * Value::PAWN;
    score += BitBoard::count(board.player_piece_bb(side, Piece::Knight)) as i32 * Value::KNIGHT;
    score += BitBoard::count(board.player_piece_bb(side, Piece::Bishop)) as i32 * Value::BISHOP;
    score += BitBoard::count(board.player_piece_bb(side, Piece::Rook)) as i32 * Value::ROOK;
    score += BitBoard::count(board.player_piece_bb(side, Piece::Queen)) as i32 * Value::QUEEN;

    score
}
