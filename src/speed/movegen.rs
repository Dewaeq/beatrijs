use crate::{
    bitboard::BitBoard,
    bitmove::{BitMove, MoveFlag},
    color::Color,
    defs::{GenType, PieceType, Square},
    gen::{
        attack::{
            attacks, bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks,
            PAWN_ATK,
        },
        between::between,
        eval::MVV_LVA,
        ray::{line, DIAGONALS, ORTHOGONALS},
    },
    movegen::pawn_push,
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
    move_list: MoveList,
    king_sq: Square,
}

impl<'a> MoveGen<'a> {
    pub fn new(
        board: &'a Board,
        hash_move: u16,
        killers: &'a [u16; 2],
        history_table: &'a HistoryTable,
    ) -> Self {
        MoveGen {
            board,
            hash_move,
            killers,
            history_table,
            move_list: MoveList::new(),
            king_sq: board.king_sq(board.turn()),
        }
    }

    pub fn simple(board: &Board) -> MoveList {
        let mut generator = MoveGen::new(board, 0, &[0; 2], &[[[0; 64]; 64]; 2]);
        generator.generate_all(GenType::All);

        generator.move_list
    }

    pub fn all(
        board: &Board,
        hash_move: u16,
        killers: &[u16; 2],
        history_table: &HistoryTable,
    ) -> MoveList {
        let mut generator = MoveGen::new(board, hash_move, killers, history_table);
        generator.generate_all(GenType::All);

        generator.move_list
    }

    pub fn captures(
        board: &Board,
        hash_move: u16,
        killers: &[u16; 2],
        history_table: &HistoryTable,
    ) -> MoveList {
        let mut generator = MoveGen::new(board, hash_move, killers, history_table);
        generator.generate_all(GenType::Captures);

        generator.move_list
    }

    fn generate_all(&mut self, gen_type: GenType) {
        let board = self.board;
        let in_check = board.in_check();
        let opponent = board.color(board.turn().opp());

        let mut mask = match gen_type {
            GenType::All => !board.color(board.turn()),
            GenType::Captures => opponent,
            _ => panic!(),
        };

        let check_mask = if in_check {
            let checker_sq = BitBoard::bit_scan_forward(board.checkers());
            between(self.king_sq, checker_sq) | BitBoard::from_sq(checker_sq)
        } else {
            !BitBoard::EMPTY
        };

        // pawn and piece moves
        if !BitBoard::more_than_one(board.checkers()) {
            // captures list should also include promotions, even if they dont capture
            let pawn_mask = mask | BitBoard::RANK_1 | BitBoard::RANK_8;

            if in_check {
                self.generate_pawn_moves::<true>(pawn_mask, check_mask);
                self.generate_knight_moves::<true>(mask, check_mask);
                self.generate_piece_moves::<true>(PieceType::Bishop, mask, check_mask);
                self.generate_piece_moves::<true>(PieceType::Rook, mask, check_mask);
                self.generate_piece_moves::<true>(PieceType::Queen, mask, check_mask);
            } else {
                self.generate_pawn_moves::<false>(pawn_mask, check_mask);
                self.generate_knight_moves::<false>(mask, check_mask);
                self.generate_piece_moves::<false>(PieceType::Bishop, mask, check_mask);
                self.generate_piece_moves::<false>(PieceType::Rook, mask, check_mask);
                self.generate_piece_moves::<false>(PieceType::Queen, mask, check_mask);
            }
        }

        // king moves
        if in_check {
            self.generate_king_moves::<true>(mask, gen_type);
        } else {
            self.generate_king_moves::<false>(mask, gen_type);
        }
    }

    fn generate_king_moves<const IN_CHECK: bool>(&mut self, mask: u64, gen_type: GenType) {
        let board = self.board;
        let opponent = board.color(board.turn().opp());
        let king_moves = king_attacks(self.king_sq) & mask;
        let mut captures = king_moves & opponent;
        let mut quiets = king_moves ^ captures;

        let occupied = board.occupied() ^ BitBoard::from_sq(self.king_sq);
        while captures != 0 {
            let dest = BitBoard::pop_lsb(&mut captures);
            if !is_square_attacked(board, dest, self.board.turn().opp(), occupied) {
                let m = BitMove::from_flag(self.king_sq, dest, MoveFlag::CAPTURE);
                self.add_move(m);
            }
        }

        while quiets != 0 {
            let dest = BitBoard::pop_lsb(&mut quiets);
            if !is_square_attacked(board, dest, self.board.turn().opp(), occupied) {
                let m = BitMove::from_squares(self.king_sq, dest);
                self.add_move(m);
            }
        }

        let occupied = board.occupied();

        // Castling
        if !IN_CHECK && gen_type == GenType::All {
            let occ = board.occupied();

            if board.can_castle_king()
                && !BitBoard::contains(occ, self.king_sq + 1)
                && !BitBoard::contains(occ, self.king_sq + 2)
                && !is_square_attacked(board, self.king_sq + 1, board.turn().opp(), occupied)
                && !is_square_attacked(board, self.king_sq + 2, board.turn().opp(), occupied)
            {
                let m = BitMove::from_flag(self.king_sq, self.king_sq + 2, MoveFlag::CASTLE_KING);
                self.add_move(m);
            }

            if board.can_castle_queen()
                && !BitBoard::contains(occ, self.king_sq - 1)
                && !BitBoard::contains(occ, self.king_sq - 2)
                && !BitBoard::contains(occ, self.king_sq - 3)
                && !is_square_attacked(board, self.king_sq - 1, board.turn().opp(), occupied)
                && !is_square_attacked(board, self.king_sq - 2, board.turn().opp(), occupied)
            {
                let m = BitMove::from_flag(self.king_sq, self.king_sq - 2, MoveFlag::CASTLE_QUEEN);
                self.add_move(m);
            }
        }
    }

    fn generate_knight_moves<const IN_CHECK: bool>(&mut self, mask: u64, check_mask: u64) {
        let mut knights = self
            .board
            .colored_piece(PieceType::Knight, self.board.turn())
            & !self.board.pinned();

        while knights != 0 {
            let src = BitBoard::pop_lsb(&mut knights);
            let moves = knight_attacks(src) & mask & check_mask;

            self.add_moves(src, moves);
        }
    }

    fn generate_piece_moves<const IN_CHECK: bool>(
        &mut self,
        piece: PieceType,
        mask: u64,
        check_mask: u64,
    ) {
        let pieces = self.board.colored_piece(piece, self.board.turn());
        let occupied = self.board.occupied();

        let mut not_pinned = pieces & !self.board.pinned();
        while not_pinned != 0 {
            let src = BitBoard::pop_lsb(&mut not_pinned);
            let moves =
                attacks(piece, src, occupied, self.board.turn().to_player()) & mask & check_mask;
            self.add_moves(src, moves);
        }

        if !IN_CHECK {
            let mut pinned = pieces & self.board.pinned();
            while pinned != 0 {
                let src = BitBoard::pop_lsb(&mut pinned);
                let moves = attacks(piece, src, occupied, self.board.turn().to_player())
                    & mask
                    & line(self.king_sq, src);
                self.add_moves(src, moves);
            }
        }
    }

    fn generate_pawn_moves<const IN_CHECK: bool>(&mut self, mask: u64, check_mask: u64) {
        let turn = self.board.turn();
        let occ = self.board.occupied();
        let pawns = self.board.colored_piece(PieceType::Pawn, turn);
        let mut not_pinned = pawns & !self.board.pinned();

        while not_pinned != 0 {
            let src = BitBoard::pop_lsb(&mut not_pinned);

            let moves = pawn_moves(src, occ, turn) & mask & check_mask;
            self.add_pawn_moves(src, moves);
        }

        if !IN_CHECK {
            let mut pinned = pawns & self.board.pinned();

            while pinned != 0 {
                let src = BitBoard::pop_lsb(&mut pinned);

                let moves = pawn_moves(src, occ, turn) & mask & line(self.king_sq, src);
                self.add_pawn_moves(src, moves);
            }
        }

        if let Some(ep_square) = self.board.ep_square() {
            let mut ep_candidates =
                pawn_attacks(ep_square, self.board.turn().opp().to_player()) & pawns;

            while ep_candidates != 0 {
                let src = BitBoard::pop_lsb(&mut ep_candidates);

                if is_legal_ep(&self.board, src) {
                    let m = BitMove::from_flag(src, ep_square, MoveFlag::EN_PASSANT);
                    self.add_move(m);
                }
            }
        }
    }

    fn add_pawn_moves(&mut self, src: Square, moves: u64) {
        let mut promotions = moves & self.board.turn().rank_8();
        while promotions != 0 {
            let dest = BitBoard::pop_lsb(&mut promotions);
            let flag = if self.board.is_occupied(dest) {
                MoveFlag::CAPTURE
            } else {
                0
            };

            for prom_type in MoveFlag::PROMOTE_KNIGHT..=MoveFlag::PROMOTE_QUEEN {
                let mut m = BitMove::from_flag(src, dest, prom_type | flag);
                self.add_move(m);
            }
        }

        let mut others = moves & !self.board.turn().rank_8();
        while others != 0 {
            let dest = BitBoard::pop_lsb(&mut others);
            let flag = if (src - dest).abs() == 16 {
                MoveFlag::DOUBLE_PAWN_PUSH
            } else {
                0
            };

            let mut m = BitMove::from_flag(src, dest, flag);
            self.add_move(m);
        }
    }

    fn add_moves(&mut self, src: Square, mut moves: u64) {
        while moves != 0 {
            let dest = BitBoard::pop_lsb(&mut moves);
            let flag = if self.board.is_occupied(dest) {
                MoveFlag::CAPTURE
            } else {
                0
            };

            let m = BitMove::from_flag(src, dest, flag);
            self.add_move(m);
        }
    }

    fn add_move(&mut self, m: u16) {
        let score = self.score_move(m);
        self.move_list.push(m, score);
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

fn is_square_attacked(board: &Board, sq: Square, color: Color, occ: u64) -> bool {
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

const fn pawn_moves(sq: Square, blockers: u64, color: Color) -> u64 {
    let bb = BitBoard::from_sq(sq);

    let mut pushes = pawn_push(bb, color.to_player()) & !blockers;
    // Double push for pawns on home rank
    if bb & color.rank_2() != 0 {
        pushes |= pawn_push(pushes, color.to_player()) & !blockers;
    }

    let captures = pawn_attacks(sq, color.to_player()) & blockers;

    pushes | captures
}

pub fn is_legal_ep(board: &Board, src: Square) -> bool {
    let ep_square = board.ep_square().unwrap();
    let opp = board.turn().opp();
    let king_sq = board.king_sq(board.turn());

    let pawn_sq = ep_square - board.turn().pawn_dir();
    let occ = board.occupied() ^ BitBoard::from_sq(src) ^ BitBoard::from_sq(pawn_sq)
        | BitBoard::from_sq(ep_square);

    let bishop_like = board.colored_piece_like(PieceType::Bishop, opp);
    if DIAGONALS[king_sq as usize] & bishop_like != 0 {
        if bishop_attacks(king_sq, occ) & bishop_like != 0 {
            return false;
        }
    }

    let rook_like = board.colored_piece_like(PieceType::Rook, opp);
    if ORTHOGONALS[king_sq as usize] & rook_like != 0 {
        if rook_attacks(king_sq, occ) & rook_like != 0 {
            return false;
        }
    }

    return true;
}
