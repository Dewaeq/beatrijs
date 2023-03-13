use crate::{
    bitboard::BitBoard,
    bitmove::BitMove,
    board::Board,
    defs::{Piece, Player, Value},
    movelist::MoveList,
    order::pick_next_move,
};
use std::cmp;
use std::time::Instant;
use crate::utils::print_search_info;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::defs::MAX_MOVES;

const IMMEDIATE_MATE_SCORE: i32 = 100000;

pub struct Searcher {
    pub num_nodes: u64,
    pub best_move: u16,
    board: Board,
    abort: Arc<AtomicBool>,
}

impl Default for Searcher {
    fn default() -> Self {
        Searcher::new(Board::start_pos(), Arc::new(AtomicBool::new(false)))
    }
}

impl Searcher {
    pub fn new(board: Board, abort: Arc<AtomicBool>) -> Self {
        Searcher {
            num_nodes: 0,
            best_move: 0,
            board,
            abort,
        }
    }

    pub fn start(&mut self) {
        self.abort.store(false, Ordering::Relaxed);
    }

    pub fn stop(&mut self) {
        self.abort.store(true, Ordering::Relaxed);
    }

    fn should_stop(&self) -> bool {
        self.abort.load(Ordering::Relaxed)
    }

    pub fn iterate(&mut self) {
        self.start();

        // save alpha and beta for aspiration search
        let mut alpha = i32::MIN + 1;
        let mut beta = i32::MAX - 1;

        for depth in 1..MAX_MOVES {
            let prev_best_move = self.best_move;
            let mut score = self.search(depth as u8, alpha, beta);

            if self.should_stop() {
                self.best_move = prev_best_move;
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
            self.best_move = 0;
        }
    }

    pub fn search(&mut self, depth: u8, alpha: i32, beta: i32) -> i32 {
        self.num_nodes = 0;
        let start = Instant::now();
        let score = self.negamax(depth, 0, alpha, beta, false);
        let end = start.elapsed();
        let time = (end.as_secs_f64() * 1000f64) as u64;

        if !self.should_stop() {
            print_search_info(depth, score, time, self.best_move, self.num_nodes);
        }

        score
    }

    fn negamax(&mut self, mut depth: u8, ply_from_root: u8, mut alpha: i32, mut beta: i32, do_null: bool) -> i32 {
        if self.should_stop() {
            return 0;
        }

        if depth == 0 {
            return self.quiesence(alpha, beta);
        }

        self.num_nodes += 1;

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

        for i in 0..moves.size() {
            pick_next_move(&mut moves, i);
            let m = moves.get(i);

            self.board.make_move(m);
            let score = -self.negamax(depth - 1, ply_from_root + 1, -beta, -alpha, true);
            self.board.unmake_move(m);

            if score >= beta {
                if !BitMove::is_cap(m) {
                    let ply = self.board.pos.ply;
                    self.board.killers[1][ply] = self.board.killers[0][ply];
                    self.board.killers[0][ply] = m;
                }

                return beta;
            }
            if score > alpha {
                alpha = score;
                if ply_from_root == 0 {
                    self.best_move = m;
                }
            }
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
