use crate::{
    bitboard::BitBoard,
    bitmove::{BitMove, MoveFlag},
    color::Color,
    defs::{GenType, PieceType, Square},
    gen::{
        attack::{
            bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks, PAWN_ATK,
        },
        between::between,
        eval::MVV_LVA,
    },
    movelist::{self, MoveList},
    search::HistoryTable,
};

use super::board::Board;

const HASH_BONUS: i32 = 9_000_000;
const PROMOTE_BONUS: i32 = 7_000_000;
const GOOD_CAPTURE_BONUS: i32 = 6_000_000;
const KILLER_1_BONUS: i32 = 5_000_000;
const KILLER_2_BONUS: i32 = 4_000_000;
const BAD_CAPTURE_BONUS: i32 = -1_000_000;

pub struct MoveGen<'a> {
    board: &'a Board,
    hash_move: u16,
    killers: &'a [u16; 2],
    history_table: &'a HistoryTable,
}

impl MoveGen<'_> {
    pub fn all(
        board: &Board,
        hash_move: u16,
        killers: &[u16; 2],
        history_table: &HistoryTable,
    ) -> MoveList {
        let generator = MoveGen {
            board,
            hash_move,
            killers,
            history_table,
        };

        generator.generate_all(GenType::All)
    }

    pub fn captures(
        board: &Board,
        hash_move: u16,
        killers: &[u16; 2],
        history_table: &HistoryTable,
    ) -> MoveList {
        let generator = MoveGen {
            board,
            hash_move,
            killers,
            history_table,
        };

        generator.generate_all(GenType::Captures)
    }

    fn generate_all(&self, gen_type: GenType) -> MoveList {
        let board = self.board;

        let mut move_list = MoveList::new();
        let king_sq = board.king_sq(board.turn());
        let in_check = board.in_check();

        let mut mask = match gen_type {
            GenType::All => !board.color(board.turn()),
            GenType::Captures => board.color(board.turn().opp()),
            _ => panic!(),
        };

        if in_check {
            let checker_sq = BitBoard::bit_scan_forward(board.checkers());
            mask &= between(king_sq, checker_sq) | BitBoard::from_sq(checker_sq);
        }

        // pawn and piece moves
        if !BitBoard::more_than_one(board.checkers()) {
            todo!();
        }

        // king moves
        todo!();

        // Castling
        if !in_check && gen_type == GenType::All {
            let occ = board.occupied();

            if board.can_castle_king()
                && !BitBoard::contains(occ, king_sq + 1)
                && !BitBoard::contains(occ, king_sq + 2)
                && !is_square_attacked(board, king_sq + 1, board.turn().opp())
                && !is_square_attacked(board, king_sq + 2, board.turn().opp())
            {
                let m = BitMove::from_flag(king_sq, king_sq + 2, MoveFlag::CASTLE_KING);
            }

            if board.can_castle_queen()
                && !BitBoard::contains(occ, king_sq - 1)
                && !BitBoard::contains(occ, king_sq - 2)
                && !BitBoard::contains(occ, king_sq - 3)
                && !is_square_attacked(board, king_sq - 1, board.turn().opp())
                && !is_square_attacked(board, king_sq - 2, board.turn().opp())
            {
                let m = BitMove::from_flag(king_sq, king_sq + 2, MoveFlag::CASTLE_QUEEN);
            }
        }

        move_list
    }

    fn add_move(&self, m: u16, move_list: &mut MoveList) {
        let score = self.score_move(m);
        move_list.push(m, score);
    }

    fn score_move(&self, m: u16) -> i32 {
        let board = self.board;
        let (src, dest) = (BitMove::src(m), BitMove::dest(m));

        if m == self.hash_move {
            HASH_BONUS
        } else if BitMove::is_prom(m) {
            PROMOTE_BONUS
        } else if BitMove::is_cap(m) {
            let mvv_lva = if BitMove::is_ep(m) {
                MVV_LVA[0][0]
            } else {
                let move_piece = board.piece_on(BitMove::src(m));
                let cap_piece = board.piece_on(BitMove::dest(m));
                MVV_LVA[move_piece.as_usize()][cap_piece.as_usize()]
            };

            if board.see_ge(m, -130) {
                GOOD_CAPTURE_BONUS + mvv_lva
            } else {
                BAD_CAPTURE_BONUS + mvv_lva
            }
        } else if m == self.killers[0] {
            KILLER_1_BONUS
        } else if m == self.killers[1] {
            KILLER_2_BONUS
        } else {
            self.history_table[board.turn().as_usize()][src as usize][dest as usize]
        }
    }
}

fn is_square_attacked(board: &Board, sq: Square, color: Color) -> bool {
    if pawn_attacks(sq, color.opp().to_player()) & board.colored_piece(PieceType::Pawn, color) != 0
    {
        return true;
    }
    if knight_attacks(sq) & board.colored_piece(PieceType::Knight, color) != 0 {
        return true;
    }

    if king_attacks(sq) & board.colored_piece(PieceType::King, color) != 0 {
        return true;
    }

    let occ = board.occupied();
    if bishop_attacks(sq, occ) & board.colored_piece_like(PieceType::Bishop, color) != 0 {
        return true;
    }
    if rook_attacks(sq, occ) & board.colored_piece_like(PieceType::Rook, color) != 0 {
        return true;
    }

    false
}

pub fn attackers(board: &Board, sq: Square, occ: u64) -> u64 {
    pawn_attacks(sq, Color::White.to_player()) & board.colored_piece(PieceType::Pawn, Color::Black)
        | pawn_attacks(sq, Color::Black.to_player())
            & board.colored_piece(PieceType::Pawn, Color::White)
        | knight_attacks(sq) & board.pieces(PieceType::Knight)
        | king_attacks(sq) & board.pieces(PieceType::King)
        | bishop_attacks(sq, occ) & board.piece_like(PieceType::Bishop)
        | rook_attacks(sq, occ) & board.piece_like(PieceType::Rook)
}
