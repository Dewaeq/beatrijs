use std::mem::{size_of, size_of_val};

use crate::{
    bitmove::BitMove,
    board::Board,
    defs::{Depth, Piece, PieceType, Player, Score},
    movelist::MoveList,
    search::MAX_STACK_SIZE,
};

pub struct Heuristics {
    pub history: [[[Score; 64]; 64]; 2],
    pub capture: [[[Score; 6]; 64]; 12],
    pub killers: [[u16; 2]; MAX_STACK_SIZE],
    //pub continuation: [[[[Score; 64]; 12]; 64]; 12],
}

impl Heuristics {
    pub fn new() -> Self {
        Heuristics {
            history: [[[0; 64]; 64]; 2],
            capture: [[[0; 6]; 64]; 12],
            killers: [[0; 2]; MAX_STACK_SIZE],
            //continuation: [[[[0; 64]; 12]; 64]; 12],
        }
    }

    pub fn clear(&mut self) {
        unsafe {
            let ptr = self.history.as_mut_ptr();
            ptr.write_bytes(0, self.history.len());

            //let ptr = self.continuation.as_mut_ptr();
            //ptr.write_bytes(0, size_of_val(&self.continuation));

            let ptr = self.capture.as_mut_ptr();
            ptr.write_bytes(0, self.capture.len());
        }
    }

    pub fn add_killer(&mut self, killer: u16, ply: usize) {
        self.killers[ply][1] = self.killers[ply][0];
        self.killers[ply][0] = killer;
    }

    pub fn update(
        &mut self,
        board: &Board,
        depth: Depth,
        best_move: u16,
        quiets: MoveList,
        noisy: MoveList,
        quiets_tried: &[Option<u16>],
    ) {
        if !BitMove::is_cap(best_move) {
            let (src, dest) = BitMove::to_squares(best_move);
            self.history[board.turn.as_usize()][src as usize][dest as usize] +=
                (depth * depth) as Score;
            for m in quiets_tried {
                let m_src = BitMove::src(m.unwrap()) as usize;
                let m_dest = BitMove::dest(m.unwrap()) as usize;
                self.history[board.turn.as_usize()][m_src][m_dest] -= (depth * depth) as Score;
            }
        }

        return;

        let bonus = (16 * (depth + 1) * (depth + 1)).min(1200) as Score;

        if BitMove::is_tactical(best_move) {
            self.update_capture(board, best_move, bonus);
        } else {
            self.update_history(board, best_move, bonus);
            self.update_continuation(board, best_move, bonus);

            for m in quiets {
                if m == best_move {
                    continue;
                }

                //self.update_history(board, m, -bonus);
                self.update_continuation(board, m, -bonus);
            }
        }

        for m in noisy {
            if m == best_move {
                continue;
            }

            self.update_capture(board, m, -bonus);
        }
    }

    fn update_history(&mut self, board: &Board, m: u16, bonus: Score) {
        let (src, dest) = BitMove::to_squares(m);
        let scaled =
            bonus - bonus.abs() * self.get_history(board.turn, src as usize, dest as usize) / 32768;
        self.history[board.turn.as_usize()][src as usize][dest as usize] += scaled;
    }

    fn update_capture(&mut self, board: &Board, m: u16, bonus: Score) {
        let (src, dest) = BitMove::to_squares(m);
        let piece = board.piece(src);
        let captured = if BitMove::is_ep(m) {
            PieceType::Pawn
        }
        // Promotion without capture
        else if !BitMove::is_cap(m) {
            PieceType::Pawn
        } else {
            board.piece_type(dest)
        };

        let scaled = bonus - bonus.abs() * self.get_capture(piece, dest as usize, captured) / 32768;
        self.capture[piece.as_usize()][dest as usize][captured.as_usize()] += scaled;
    }

    /// TODO
    fn update_continuation(&mut self, board: &Board, m: u16, bonus: Score) {}

    pub fn get_heuristic(&self, board: &Board, m: u16) -> Score {
        let (src, dest) = BitMove::to_squares(m);
        if !BitMove::is_tactical(m) {
            self.get_history(board.turn, src as usize, dest as usize)
        } else {
            let piece = board.piece(src);
            let captured = if BitMove::is_ep(m) {
                PieceType::Pawn
            }
            // Promotion without capture
            else if !BitMove::is_cap(m) {
                PieceType::Pawn
            } else {
                board.piece_type(dest)
            };

            self.get_capture(piece, dest as usize, captured)
        }
    }

    pub fn get_history(&self, turn: Player, src: usize, dest: usize) -> Score {
        self.history[turn.as_usize()][src][dest]
    }

    pub fn get_capture(&self, piece: Piece, dest: usize, captured: PieceType) -> Score {
        self.capture[piece.as_usize()][dest][captured.as_usize()]
    }
}
