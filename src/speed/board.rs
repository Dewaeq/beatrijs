use crate::{
    bitboard::BitBoard,
    bitmove::{BitMove, MoveFlag},
    color::Color,
    defs::{Castling, Piece, PieceType, Square, NUM_PIECES, NUM_SIDES},
    gen::{
        attack::{knight_attacks, pawn_attacks},
        between::between,
        ray::{DIAGONALS, ORTHOGONALS},
    },
    zobrist::Zobrist,
};

#[derive(Clone, Copy)]
pub struct Board {
    pieces: [u64; NUM_PIECES],
    colors: [u64; NUM_SIDES],
    occupied: u64,
    pinned: u64,
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

        target.pinned = 0;
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
        let mut attackers = DIAGONALS[opp_king_sq as usize] & target.piece_like(PieceType::Bishop);
        attackers |= ORTHOGONALS[opp_king_sq as usize] & target.piece_like(PieceType::Queen);
        attackers &= target.color(self.turn);

        while attackers != 0 {
            let sq = BitBoard::pop_lsb(&mut attackers);
            let between = between(sq, opp_king_sq);

            if between == 0 {
                target.checkers |= BitBoard::from_sq(sq);
            } else if BitBoard::only_one(between) {
                target.pinned ^= between;
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

    pub fn color(&self, color: Color) -> u64 {
        unsafe { *self.colors.get_unchecked(color.as_usize()) }
    }

    pub fn colored_piece(&self, piece: PieceType, color: Color) -> u64 {
        self.pieces(piece) & self.color(color)
    }

    fn king_sq(&self, color: Color) -> Square {
        BitBoard::to_sq(self.colored_piece(PieceType::King, color))
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
}
