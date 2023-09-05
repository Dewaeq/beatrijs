use crate::{
    bitboard::BitBoard,
    bitmove::{BitMove, MoveFlag},
    color::Color,
    defs::{Castling, Piece, PieceType, Score, Square, NUM_PIECES, NUM_SIDES},
    gen::{
        attack::{bishop_attacks, knight_attacks, pawn_attacks, rook_attacks},
        between::between,
        ray::{DIAGONALS, ORTHOGONALS},
    },
    zobrist::Zobrist,
};

use super::movegen::attackers;

#[derive(Clone, Copy)]
pub struct Board {
    pieces: [u64; NUM_PIECES],
    colors: [u64; NUM_SIDES],
    occupied: u64,
    pinned_diag: u64,
    pinned_ortho: u64,
    checkers: u64,
    turn: Color,
    hash: u64,
    ep_square: Option<Square>,
    castling: u8,
    fifty_move: u8,
    /// Half move counter, doesn't reset after search
    his_ply: u8,
}

impl Board {
    pub fn make_move(&self, m: u16) -> Board {
        let mut target = *self;

        target.pinned_ortho = 0;
        target.pinned_diag = 0;
        target.checkers = 0;

        // Reset ep square
        if let Some(ep_square) = self.ep_square {
            target.ep_square = None;
            target.hash ^= Zobrist::ep(ep_square % 8);
        }

        let opp = self.turn.opp();
        let opp_king_sq = self.king_sq(opp);

        let is_castle = BitMove::is_castle(m);
        let src = BitMove::src(m);
        let dest = BitMove::dest(m);

        let src_bb = BitBoard::from_sq(src);
        let dest_bb = BitBoard::from_sq(dest);
        let piece = self.piece_on(src);
        let captured = self.piece_on(dest);

        target.toggle(piece, src_bb, self.turn);
        target.toggle(piece, dest_bb, self.turn);

        // Remove the captured piece
        if captured != PieceType::None {
            target.toggle(captured, dest_bb, opp);
        }

        // Handle edge cases
        if piece == PieceType::Pawn {
            if BitMove::is_prom(m) {
                // Remove the pawn from the 1/8th rank, as it will
                // promote to a piece
                target.toggle(PieceType::Pawn, dest_bb, self.turn);

                let prom_type = BitMove::prom_type(BitMove::flag(m));
                target.toggle(prom_type, dest_bb, self.turn);

                if prom_type == PieceType::Knight {
                    target.checkers |= knight_attacks(opp_king_sq) & dest_bb;
                }
            } else {
                target.checkers |= pawn_attacks(opp_king_sq, opp.to_player()) & dest_bb;

                if BitMove::flag(m) == MoveFlag::DOUBLE_PAWN_PUSH {
                    target.set_ep(dest - self.turn.pawn_dir());
                } else if Some(dest) == self.ep_square {
                    target.toggle(
                        PieceType::Pawn,
                        BitBoard::from_sq(dest - self.turn.pawn_dir()),
                        opp,
                    );
                }
            }
        } else if piece == PieceType::Knight {
            target.checkers |= knight_attacks(opp_king_sq) & dest_bb;
        }
        // TODO: does first checking if [piece] == PieceType::King improve performance?
        else if BitMove::is_castle(m) {
            let king_side = dest % 8 < 4;

            let (rook_src, rook_dest) = if king_side {
                (dest + 1, dest - 1)
            } else {
                (dest - 2, dest + 1)
            };

            target.toggle(PieceType::Rook, BitBoard::from_sq(rook_src), self.turn);
            target.toggle(PieceType::Rook, BitBoard::from_sq(rook_dest), self.turn);
        }

        // Update checkers and pinners
        let mut attackers = DIAGONALS[opp_king_sq as usize]
            & target.colored_piece_like(PieceType::Bishop, self.turn);

        while attackers != 0 {
            let sq = BitBoard::pop_lsb(&mut attackers);
            let between = between(sq, opp_king_sq);

            if between == 0 {
                target.checkers |= BitBoard::from_sq(sq);
            } else if BitBoard::only_one(between) {
                target.pinned_diag ^= between;
            }
        }

        attackers = ORTHOGONALS[opp_king_sq as usize]
            & target.colored_piece_like(PieceType::Queen, self.turn);

        while attackers != 0 {
            let sq = BitBoard::pop_lsb(&mut attackers);
            let between = between(sq, opp_king_sq);

            if between == 0 {
                target.checkers |= BitBoard::from_sq(sq);
            } else if BitBoard::only_one(between) {
                target.pinned_ortho ^= between;
            }
        }

        target.update_castling(src, dest);

        target.turn = opp;
        target.hash ^= Zobrist::side();

        target
    }

    pub fn piece_on(&self, sq: Square) -> PieceType {
        let bb = BitBoard::from_sq(sq);

        if self.occupied & bb == 0 {
            PieceType::None
        } else if self.pieces(PieceType::Pawn) & bb != 0 {
            PieceType::Pawn
        } else if self.pieces(PieceType::Knight) & bb != 0 {
            PieceType::Knight
        } else if self.pieces(PieceType::Bishop) & bb != 0 {
            PieceType::Bishop
        } else if self.pieces(PieceType::Rook) & bb != 0 {
            PieceType::Rook
        } else if self.pieces(PieceType::Queen) & bb != 0 {
            PieceType::Queen
        } else if self.pieces(PieceType::King) & bb != 0 {
            PieceType::King
        } else {
            unreachable!()
        }
    }

    pub const fn turn(&self) -> Color {
        self.turn
    }

    pub const fn checkers(&self) -> u64 {
        self.checkers
    }

    pub const fn in_check(&self) -> bool {
        self.checkers != 0
    }

    pub const fn can_castle_queen(&self) -> bool {
        match self.turn {
            Color::White => self.castling & Castling::WQ != 0,
            Color::Black => self.castling & Castling::BQ != 0,
        }
    }

    pub const fn can_castle_king(&self) -> bool {
        match self.turn {
            Color::White => self.castling & Castling::WK != 0,
            Color::Black => self.castling & Castling::BK != 0,
        }
    }

    pub fn pieces(&self, piece: PieceType) -> u64 {
        unsafe { *self.pieces.get_unchecked(piece.as_usize()) }
    }

    pub fn piece_like(&self, piece: PieceType) -> u64 {
        match piece {
            PieceType::Bishop => self.pieces(PieceType::Bishop) | self.pieces(PieceType::Queen),
            PieceType::Rook => self.pieces(PieceType::Rook) | self.pieces(PieceType::Queen),
            _ => panic!(),
        }
    }

    pub fn colored_piece_like(&self, piece: PieceType, color: Color) -> u64 {
        self.piece_like(piece) & self.color(color)
    }

    pub fn color(&self, color: Color) -> u64 {
        unsafe { *self.colors.get_unchecked(color.as_usize()) }
    }

    pub fn colored_piece(&self, piece: PieceType, color: Color) -> u64 {
        self.pieces(piece) & self.color(color)
    }

    pub const fn occupied(&self) -> u64 {
        self.occupied
    }

    pub fn king_sq(&self, color: Color) -> Square {
        BitBoard::to_sq(self.colored_piece(PieceType::King, color))
    }

    pub const fn pinned_diag(&self) -> u64 {
        self.pinned_diag
    }

    pub const fn pinned_ortho(&self) -> u64 {
        self.pinned_ortho
    }

    pub const fn pinned(&self) -> u64 {
        self.pinned_diag | self.pinned_ortho
    }

    fn toggle(&mut self, piece: PieceType, bb: u64, color: Color) {
        self.hash ^= Zobrist::piece(color.to_player(), piece, BitBoard::to_sq(bb));
        self.occupied ^= bb;

        unsafe {
            *self.pieces.get_unchecked_mut(piece.as_usize()) ^= bb;
            *self.colors.get_unchecked_mut(color.as_usize()) ^= bb;
        }
    }

    fn set_ep(&mut self, sq: Square) {
        self.ep_square = Some(sq);
        self.hash ^= Zobrist::ep(sq % 8);
    }

    fn update_castling(&mut self, src: Square, dest: Square) {
        self.hash ^= Zobrist::castle(self.castling);

        self.castling &= Castling::RIGHTS[src as usize];
        self.castling &= Castling::RIGHTS[dest as usize];

        self.hash ^= Zobrist::castle(self.castling);
    }

    pub fn see_ge(&self, m: u16, threshold: Score) -> bool {
        if BitMove::is_prom(m) {
            return true;
        }

        let src = BitMove::src(m);
        let dest = BitMove::dest(m);

        let captured = self.piece_on(dest);
        let mut balance = captured.mg_value() - threshold;

        if balance < 0 {
            return false;
        }

        let attacker = self.piece_on(src);
        balance -= attacker.mg_value();

        if balance >= 0 {
            return true;
        }

        let bishop_like = self.piece_like(PieceType::Bishop);
        let rook_like = self.piece_like(PieceType::Rook);
        let mut occ = self.occupied ^ BitBoard::from_sq(src) ^ BitBoard::from_sq(dest);
        let mut attackers = attackers(&self, dest, occ);

        let mut stm = self.turn.opp();

        loop {
            attackers &= occ;

            let stm_attackers = attackers & self.color(stm);
            if stm_attackers == 0 {
                break;
            }

            let next_piece = self.smallest_attacker(stm_attackers);

            stm = stm.opp();
            balance = -balance - 1 - next_piece.mg_value();

            if balance >= 0 {
                if next_piece == PieceType::King && (attackers & self.color(stm)) != 0 {
                    stm = stm.opp();
                }
                break;
            }

            let attacker_bb = stm_attackers & self.pieces(next_piece);
            let attacker_sq = BitBoard::bit_scan_forward(attacker_bb);
            occ ^= BitBoard::from_sq(attacker_sq);

            if next_piece == PieceType::Pawn
                || next_piece == PieceType::Bishop
                || next_piece == PieceType::Queen
            {
                attackers |= bishop_attacks(dest, occ) & bishop_like;
            }

            if next_piece == PieceType::Rook || next_piece == PieceType::Queen {
                attackers |= rook_attacks(dest, occ) & rook_like;
            }
        }

        stm != self.turn
    }

    fn smallest_attacker(&self, stm_attackers: u64) -> PieceType {
        let pieces = [
            PieceType::Pawn,
            PieceType::King,
            PieceType::Bishop,
            PieceType::Rook,
            PieceType::Queen,
            PieceType::King,
        ];

        for piece in pieces {
            if self.pieces(piece) & stm_attackers != 0 {
                return piece;
            }
        }

        panic!()
    }
}
