use crate::{
    board::Board,
    defs::{PieceType, Player},
    movelist::MoveList,
};
use std::cmp;

const IMMEDIATE_MATE_SCORE: i32 = 100000;

pub struct Searcher {
    pub num_nodes: u32,
    board: Board,
}

impl Searcher {
    pub fn new(board: Board) -> Self {
        Searcher {
            num_nodes: 0,
            board,
        }
    }

    pub fn search(&mut self, depth: u8) -> i32 {
        self.num_nodes = 0;
        self.negamax(depth, 0, i32::MIN + 1, i32::MAX - 1)
    }

    fn negamax(&mut self, depth: u8, ply_from_root: u8, mut alpha: i32, mut beta: i32) -> i32 {
        self.num_nodes += 1;

        if ply_from_root > 0 {
            alpha = i32::max(-IMMEDIATE_MATE_SCORE + ply_from_root as i32, alpha);
            beta = i32::min(IMMEDIATE_MATE_SCORE - ply_from_root as i32, beta);

            if alpha >= beta {
                return alpha;
            }
        }

        let moves = MoveList::legal(&mut self.board);
        if moves.is_empty() {
            return -IMMEDIATE_MATE_SCORE + ply_from_root as i32;
        }

        if depth == 0 {
            return self.quiesence(alpha, beta);
        }

        for m in moves {
            let old_board = *&self.board;
            self.board.make_move(m);

            let score = -self.negamax(depth - 1, ply_from_root + 1, -beta, -alpha);

            self.board = old_board;

            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }

    fn quiesence(&mut self, mut alpha: i32, beta: i32) -> i32 {
        self.num_nodes += 1;

        let stand_pat = evaluate(&self.board);
        if stand_pat >= beta {
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        let moves = MoveList::quiet(&mut self.board);
        for m in moves {
            let old_board = *&self.board;
            self.board.make_move(m);
            let score = -self.quiesence(-beta, -alpha);
            self.board = old_board;

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
    score += board.player_piece_bb(side, PieceType::Pawn).count_ones() * 100;
    score += board.player_piece_bb(side, PieceType::Knight).count_ones() * 300;
    score += board.player_piece_bb(side, PieceType::Bishop).count_ones() * 320;
    score += board.player_piece_bb(side, PieceType::Rook).count_ones() * 520;
    score += board.player_piece_bb(side, PieceType::Queen).count_ones() * 900;

    score as i32
}