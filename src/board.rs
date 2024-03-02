use std::cmp;

use crate::{
    bitboard::BitBoard,
    bitmove::{BitMove, MoveFlag},
    defs::{
        Castling, Piece, PieceType, Player, Score, Square, BLACK_IDX, DARK_SQUARES,
        FEN_START_STRING, LIGHT_SQUARES, MAX_MOVES, MG_VALUE, NUM_PIECES, NUM_SIDES, NUM_SQUARES,
        WHITE_IDX,
    },
    gen::{
        attack::{attacks, bishop_attacks, knight_attacks, pawn_attacks, rook_attacks},
        between::between,
    },
    history::History,
    movegen::{attackers_to, smallest_attacker},
    position::Position,
    search::MAX_STACK_SIZE,
    utils::{square_from_string, square_to_string},
    zobrist::Zobrist,
};

#[derive(Clone, Copy)]
pub struct Board {
    pub turn: Player,
    pub piece_bb: [u64; NUM_PIECES],
    pub side_bb: [u64; NUM_SIDES],
    pub pieces: [Piece; NUM_SQUARES],
    pub pos: Position,
    pub history: History,
    /// Quiet moves that caused a beta-cutoff, used for ordering
    pub killers: [[u16; MAX_STACK_SIZE]; 2],
}

/// Getter methods
impl Board {
    pub const fn key(&self) -> u64 {
        self.pos.key
    }

    pub const fn piece(&self, square: Square) -> Piece {
        assert!(square < 64);
        self.pieces[square as usize]
    }

    /// Get the [`PieceType`] of the piece on the provided square
    pub const fn piece_type(&self, square: Square) -> PieceType {
        assert!(square < 64);
        self.pieces[square as usize].t
    }

    pub const fn occ_bb(&self) -> u64 {
        self.side_bb[0] | self.side_bb[1]
    }

    pub const fn cur_player_bb(&self) -> u64 {
        self.player_bb(self.turn)
    }

    pub const fn player_bb(&self, side: Player) -> u64 {
        unsafe {
            match side {
                Player::White => self.side_bb[WHITE_IDX],
                _ => self.side_bb[BLACK_IDX],
            }
        }
    }

    pub const fn piece_bb(&self, piece: PieceType) -> u64 {
        self.piece_bb[piece.as_usize()]
    }

    /// Get a piece-like bitboard.
    ///
    /// Eg `get_piece_like_bb(PieceType::Bishop)` returns queen and bishop bitboards combined
    pub const fn piece_like_bb(&self, piece_like: PieceType) -> u64 {
        self.piece_bb(piece_like) | self.piece_bb(PieceType::Queen)
    }

    pub const fn player_piece_like_bb(&self, side: Player, piece_like: PieceType) -> u64 {
        (self.piece_bb(piece_like) | self.piece_bb(PieceType::Queen)) & self.player_bb(side)
    }

    pub const fn player_piece_bb(&self, side: Player, piece: PieceType) -> u64 {
        let piece_bb = self.piece_bb(piece);
        let side_bb = self.player_bb(side);
        piece_bb & side_bb
    }

    pub const fn cur_king_square(&self) -> Square {
        let bb = self.player_piece_bb(self.turn, PieceType::King);
        BitBoard::bit_scan_forward(bb)
    }

    pub const fn king_square(&self, side: Player) -> Square {
        let bb = self.player_piece_bb(side, PieceType::King);
        BitBoard::bit_scan_forward(bb)
    }

    pub const fn in_check(&self) -> bool {
        self.pos.checkers_bb != 0
    }

    pub const fn gives_check(&self, m: u16) -> bool {
        let src = BitMove::src(m);
        let dest = BitMove::dest(m);
        let from_bb = BitBoard::from_sq(src);
        let to_bb = BitBoard::from_sq(dest);
        let piece = self.piece_type(src);

        // Direct check
        if self.pos.check_squares[piece.as_usize()] & to_bb != 0 {
            return true;
        }

        // Discovered check
        let opp = self.turn.opp();
        let opp_king_sq = self.king_square(opp);
        if self.blockers(opp) & from_bb != 0 && !BitBoard::triple_aligned(src, dest, opp_king_sq) {
            return true;
        }

        let flag = BitMove::flag(m);
        if BitMove::is_normal(m) {
            return false;
        }

        let opp_king_bb = BitBoard::from_sq(opp_king_sq);

        if BitMove::is_prom(m) {
            let prom_type = BitMove::prom_type(flag);
            let occ = self.occ_bb() ^ from_bb;

            return attacks(prom_type, dest, occ, self.turn) & opp_king_bb != 0;
        }

        if BitMove::is_ep(m) {
            let captured_sq = dest - self.turn.pawn_dir();
            let occ = (self.occ_bb() ^ from_bb ^ BitBoard::from_sq(captured_sq)) | to_bb;

            let bishop_attacks = bishop_attacks(opp_king_sq, occ)
                & self.player_piece_like_bb(self.turn, PieceType::Bishop);
            if bishop_attacks != 0 {
                return true;
            }

            let rook_attacks = rook_attacks(opp_king_sq, occ)
                & self.player_piece_like_bb(self.turn, PieceType::Rook);
            if rook_attacks != 0 {
                return true;
            }
        }

        if BitMove::is_castle(m) {
            let rook_dest = if dest > src { dest - 1 } else { dest + 1 };
            let occ = self.occ_bb() ^ from_bb ^ to_bb;

            return rook_attacks(rook_dest, occ) & opp_king_bb != 0;
        }

        return false;
    }

    pub const fn can_ep(&self) -> bool {
        self.pos.ep_square < 64
    }

    pub const fn ep_file(&self) -> Square {
        self.pos.ep_square % 8
    }

    pub const fn can_castle_queen(&self, side: Player) -> bool {
        match side {
            Player::White => self.pos.castling & Castling::WQ != 0,
            Player::Black => self.pos.castling & Castling::BQ != 0,
        }
    }

    pub const fn can_castle_king(&self, side: Player) -> bool {
        match side {
            Player::White => self.pos.castling & Castling::WK != 0,
            Player::Black => self.pos.castling & Castling::BK != 0,
        }
    }

    pub const fn can_castle(&self, side: Player) -> bool {
        match side {
            Player::White => self.pos.castling & Castling::WHITE_ALL != 0,
            Player::Black => self.pos.castling & Castling::BLACK_ALL != 0,
        }
    }

    pub const fn pawns_on_sq_color(&self, side: Player, sq: Square) -> u64 {
        let pawns = self.player_piece_bb(side, PieceType::Pawn);
        if sq % 2 == 0 {
            pawns & DARK_SQUARES
        } else {
            pawns & LIGHT_SQUARES
        }
    }

    pub const fn has_non_pawns(&self, side: Player) -> bool {
        self.player_piece_bb(side, PieceType::Knight) != 0
            || self.player_piece_bb(side, PieceType::Bishop) != 0
            || self.player_piece_bb(side, PieceType::Rook) != 0
            || self.player_piece_bb(side, PieceType::Queen) != 0
    }

    pub const fn has_big_piece(&self, side: Player) -> bool {
        self.player_piece_bb(side, PieceType::Bishop) != 0
            || self.player_piece_bb(side, PieceType::Rook) != 0
            || self.player_piece_bb(side, PieceType::Queen) != 0
    }

    pub const fn blockers(&self, side: Player) -> u64 {
        self.pos.king_blockers[side.as_usize()]
    }

    pub fn slider_blockers(&self, us_bb: u64, opp_bb: u64, sq: Square) -> (u64, u64) {
        let mut blockers = 0;
        let mut pinners = 0;

        let mut snipers = ((bishop_attacks(sq, 0) & self.piece_like_bb(PieceType::Bishop))
            | (rook_attacks(sq, 0) & self.piece_like_bb(PieceType::Rook)))
            & opp_bb;
        let occ = self.occ_bb() ^ snipers;

        while snipers != 0 {
            let sniper_sq = BitBoard::pop_lsb(&mut snipers);
            let b = between(sq, sniper_sq) & occ;

            if b != 0 && !BitBoard::more_than_one(b) {
                blockers |= b;
                if b & us_bb != 0 {
                    pinners |= BitBoard::from_sq(sniper_sq);
                }
            }
        }

        (blockers, pinners)
    }
}

/// Setter methods
impl Board {
    /// Calculate checkers, pinners and pinned pieces
    pub fn set_check_info(&mut self) {
        let opp = self.turn.opp();
        let occ = self.occ_bb();
        let us_bb = self.cur_player_bb();
        let opp_bb = self.player_bb(opp);
        let king_sq = self.cur_king_square();
        let opp_king_sq = self.king_square(opp);

        self.pos.checkers_bb = attackers_to(self, king_sq, occ) & opp_bb;

        let (w_pieces, b_pieces, w_king_sq, b_king_sq) = match self.turn {
            Player::White => (us_bb, opp_bb, king_sq, opp_king_sq),
            _ => (opp_bb, us_bb, opp_king_sq, king_sq),
        };

        let (w_blockers, b_pinners) = self.slider_blockers(w_pieces, b_pieces, w_king_sq);
        let (b_blockers, w_pinners) = self.slider_blockers(b_pieces, w_pieces, b_king_sq);

        self.pos.king_blockers = [w_blockers, b_blockers];
        self.pos.pinners = [w_pinners, b_pinners];

        self.set_check_squares(PieceType::Pawn, pawn_attacks(opp_king_sq, self.turn.opp()));
        self.set_check_squares(PieceType::Knight, knight_attacks(opp_king_sq));

        self.set_check_squares(
            PieceType::Bishop,
            attacks(PieceType::Bishop, opp_king_sq, occ, self.turn),
        );
        self.set_check_squares(
            PieceType::Rook,
            attacks(PieceType::Rook, opp_king_sq, occ, self.turn),
        );
        self.set_check_squares(
            PieceType::Queen,
            self.check_squares(PieceType::Bishop) | self.check_squares(PieceType::Rook),
        );
    }

    fn set_check_squares(&mut self, piece: PieceType, bb: u64) {
        unsafe { *self.pos.check_squares.get_unchecked_mut(piece.as_usize()) = bb }
    }

    const fn check_squares(&self, piece: PieceType) -> u64 {
        self.pos.check_squares[piece.as_usize()]
    }

    /// Removes castling permissions for the given side
    pub fn disable_castling(&mut self, side: Player) {
        match side {
            Player::White => self.pos.castling &= Castling::BLACK_ALL,
            Player::Black => self.pos.castling &= Castling::WHITE_ALL,
        }
    }

    pub fn make_move(&mut self, m: u16) {
        let src = BitMove::src(m);
        let dest = BitMove::dest(m);
        let flag = BitMove::flag(m);
        let is_cap = BitMove::is_cap(m);
        let is_prom = BitMove::is_prom(m);
        let is_castle = BitMove::is_castle(m);
        let is_ep = BitMove::is_ep(m);
        let piece = self.piece_type(src);
        let opp = self.turn.opp();
        let old_castle = self.pos.castling;

        assert!(piece != PieceType::None);
        assert!(src != dest);

        self.history.push(self.pos);
        self.pos.last_move = Some(m);

        // Remove all castling rights for the moving side when a king move occurs
        if piece == PieceType::King {
            self.disable_castling(self.turn);
        }

        // Normal captures
        if is_cap && !is_ep {
            let cap_pt = self.piece_type(dest);
            self.pos.captured_piece = cap_pt;
            self.remove_piece(opp, cap_pt, dest);

            // target.pos.key ^= Zobrist::piece(opp, cap_pt, dest);
        }

        // EP capture
        if self.can_ep() {
            if is_ep {
                let ep_pawn_sq = self.pos.ep_square - self.turn.pawn_dir();
                self.remove_piece(opp, PieceType::Pawn, ep_pawn_sq);
                // target.pos.key ^= Zobrist::piece(opp, PieceType::Pawn, dest);
            }

            // target.pos.key ^= Zobrist::ep(self.ep_file());
            self.clear_ep();
        }

        if flag == MoveFlag::DOUBLE_PAWN_PUSH {
            self.set_ep(dest - self.turn.pawn_dir());
            // target.pos.key ^= Zobrist::ep(self.ep_file());
        }

        // Castling
        if is_castle {
            let rook_sq;
            let rook_target_sq;

            if flag == MoveFlag::CASTLE_KING {
                rook_sq = self.turn.castle_king_sq() + 1;
                rook_target_sq = self.turn.castle_king_sq() - 1;
            } else {
                rook_sq = self.turn.castle_queen_sq() - 2;
                rook_target_sq = self.turn.castle_queen_sq() + 1;
            }

            self.remove_piece(self.turn, PieceType::Rook, rook_sq);
            self.add_piece(self.turn, PieceType::Rook, rook_target_sq);

            // target.pos.key ^= Zobrist::piece(self.turn, PieceType::Rook, rook_sq);
            // target.pos.key ^= Zobrist::piece(self.turn, PieceType::Rook, rook_target_sq);
        }

        // Promotion
        if is_prom {
            let prom_type = BitMove::prom_type(flag);
            self.add_piece(self.turn, prom_type, dest);
            // target.pos.key ^= Zobrist::piece(self.turn, prom_type, dest);
        } else {
            self.add_piece(self.turn, piece, dest);
            // target.pos.key ^= Zobrist::piece(self.turn, piece_type, dest);
        }

        if self.pos.castling != old_castle {
            self.pos.key ^= Zobrist::castle(self.pos.castling);
        }

        if piece == PieceType::Pawn || is_cap {
            self.pos.half_move_count = 0;
        } else {
            self.pos.half_move_count += 1;
        }

        self.pos.key ^= Zobrist::side();
        // target.pos.key ^= Zobrist::piece(self.turn, piece_type, src);

        self.remove_piece(self.turn, piece, src);
        self.set_castling_from_move(m);
        self.turn = self.turn.opp();
        self.pos.ply += 1;
        self.set_check_info();
    }

    pub fn unmake_move(&mut self, m: u16) {
        let src = BitMove::src(m);
        let dest = BitMove::dest(m);
        let flag = BitMove::flag(m);
        let is_cap = BitMove::is_cap(m);
        let is_prom = BitMove::is_prom(m);
        let is_castle = BitMove::is_castle(m);
        let is_ep = BitMove::is_ep(m);
        let piece = self.piece_type(dest);
        let opp = self.turn.opp();

        self.remove_piece(opp, piece, dest);

        if is_prom {
            self.add_piece(opp, PieceType::Pawn, src);
        } else {
            self.add_piece(opp, piece, src);
        }

        if is_ep {
            self.add_piece(self.turn, PieceType::Pawn, dest + self.turn.pawn_dir());
        } else if is_cap {
            self.add_piece(self.turn, self.pos.captured_piece, dest);
        }

        if is_castle {
            let rook_sq;
            let rook_home_sq;

            if flag == MoveFlag::CASTLE_KING {
                rook_sq = dest - 1;
                rook_home_sq = dest + 1;
            } else {
                rook_sq = dest + 1;
                rook_home_sq = dest - 2;
            }

            self.remove_piece(opp, PieceType::Rook, rook_sq);
            self.add_piece(opp, PieceType::Rook, rook_home_sq);
        }

        self.pos = self.history.pop();
        self.turn = opp;
    }

    pub fn unmake_last_move(&mut self) {
        if let Some(m) = self.pos.last_move {
            self.unmake_move(m);
        }
    }

    pub fn make_null_move(&mut self) {
        self.history.push(self.pos);

        self.pos.last_move = None;
        self.pos.ply += 1;
        self.pos.key ^= Zobrist::side();
        self.turn = self.turn.opp();
        if self.can_ep() {
            self.clear_ep();
        }
        self.set_check_info();
    }

    pub fn unmake_null_move(&mut self) {
        self.pos = self.history.pop();
        self.turn = self.turn.opp();
    }

    pub fn clear_killers(&mut self) {
        self.killers = [[0; MAX_STACK_SIZE]; 2];
    }

    pub fn see_capture(&self, m: u16) -> Score {
        if !BitMove::is_cap(m) {
            return 0;
        }

        let captured = self.piece_type(BitMove::dest(m));
        let mut new_board: Board = *self;
        new_board.make_move(m);

        MG_VALUE[captured.as_usize()] - new_board.see(BitMove::dest(m))
    }

    fn see(&mut self, dest: Square) -> Score {
        let captured = self.piece_type(dest);
        let (attacker, src) = smallest_attacker(self, dest, self.turn);

        if attacker != PieceType::None {
            self.move_piece_cheap(src, dest, attacker, captured);
            cmp::max(0, MG_VALUE[captured.as_usize()] - self.see(dest))
        } else {
            0
        }
    }

    pub fn see_approximate(&self, m: u16) -> Score {
        let src = BitMove::src(m);
        let dest = BitMove::dest(m);

        let piece = self.piece_type(src);
        let captured = if BitMove::is_ep(m) {
            PieceType::Pawn
        } else {
            self.piece_type(dest)
        };

        let mut score = captured.mg_value();

        if BitMove::is_prom(m) {
            score += BitMove::prom_type(BitMove::flag(m)).mg_value() - piece.mg_value();
        }

        score - piece.mg_value()
    }

    pub fn see_ge(&self, m: u16, threshold: Score) -> bool {
        if !BitMove::is_cap(m) || BitMove::is_ep(m) {
            return threshold <= 0;
        }

        let src = BitMove::src(m);
        let dest = BitMove::dest(m);

        let us: Player;
        let mut stm: Player;

        let piece = self.piece(src);
        let captured = self.piece_type(dest);
        if piece.is_none() {
            return false;
        } else {
            us = piece.c;
            stm = us.opp();
        }

        let mut balance = captured.mg_value() - threshold;
        if balance < 0 {
            return false;
        }

        // Recapture
        balance -= piece.t.mg_value();
        if balance >= 0 {
            return true;
        }

        // Remove the two pieces we just evaluated
        let mut occ = self.occ_bb() ^ BitBoard::from_sq(src) ^ BitBoard::from_sq(dest);
        let mut attackers = attackers_to(&self, dest, occ) & occ;
        let mut stm_attackers;
        let mut next_capture;

        loop {
            stm_attackers = attackers & self.player_bb(stm);

            if self.pos.pinners[stm.opp().as_usize()] & !occ == 0 {
                stm_attackers &= !self.pos.king_blockers[stm.as_usize()];
            }

            if stm_attackers == 0 {
                break;
            }

            next_capture = self.min_attacker(
                PieceType::Pawn,
                dest,
                stm_attackers,
                &mut occ,
                &mut attackers,
            );
            stm = stm.opp();

            balance = -balance - 1 - next_capture.mg_value();

            if balance >= 0 {
                if next_capture == PieceType::King && (attackers & self.player_bb(stm) != 0) {
                    stm = stm.opp();
                }
                break;
            }
        }

        us != stm
    }

    fn min_attacker(
        &self,
        piece: PieceType,
        to: Square,
        stm_attackers: u64,
        occ: &mut u64,
        attackers: &mut u64,
    ) -> PieceType {
        let bb = stm_attackers & self.piece_bb(piece);
        if bb == 0 {
            let np = match piece {
                PieceType::Pawn => {
                    self.min_attacker(PieceType::Knight, to, stm_attackers, occ, attackers)
                }
                PieceType::Knight => {
                    self.min_attacker(PieceType::Bishop, to, stm_attackers, occ, attackers)
                }
                PieceType::Bishop => {
                    self.min_attacker(PieceType::Rook, to, stm_attackers, occ, attackers)
                }
                PieceType::Rook => {
                    self.min_attacker(PieceType::Queen, to, stm_attackers, occ, attackers)
                }
                _ => self.min_attacker(PieceType::King, to, stm_attackers, occ, attackers),
            };

            return np;
        }

        *occ ^= BitBoard::from_sq(BitBoard::bit_scan_forward(bb));

        if piece == PieceType::Pawn || piece == PieceType::Bishop || piece == PieceType::Queen {
            *attackers |= bishop_attacks(to, *occ)
                & (self.piece_bb(PieceType::Bishop) | self.piece_bb(PieceType::Queen));
        }
        if piece == PieceType::Rook || piece == PieceType::Queen {
            *attackers |= rook_attacks(to, *occ)
                & (self.piece_bb(PieceType::Rook) | self.piece_bb(PieceType::Queen));
        }

        *attackers &= *occ;

        piece
    }

    fn move_piece_cheap(
        &mut self,
        src: Square,
        dest: Square,
        piece: PieceType,
        captured: PieceType,
    ) {
        self.remove_piece(self.turn, piece, src);
        self.remove_piece(self.turn.opp(), captured, dest);
        self.add_piece(self.turn, piece, dest);
        self.turn = self.turn.opp();
    }

    pub fn set_castling_from_move(&mut self, m: u16) {
        let src = BitMove::src(m);
        let dest = BitMove::dest(m);

        // White castle queen-side
        if src == 0 || dest == 0 {
            self.pos.castling &= 0b1110;
        }
        // White castle king-side
        if src == 7 || dest == 7 {
            self.pos.castling &= 0b1101;
        }
        // Black castle queen-side
        if src == 56 || dest == 56 {
            self.pos.castling &= 0b1011;
        }
        // Black castle king-side
        if src == 63 || dest == 63 {
            self.pos.castling &= 0b0111;
        }
    }

    /// Calling this function is slower than manually setting the ep_square
    /// TODO: investigate this further
    pub fn set_ep(&mut self, ep_square: Square) {
        self.pos.ep_square = ep_square;
        self.pos.key ^= Zobrist::ep(self.ep_file());
    }

    /// It's faster to call this function even if the ep_square is already cleared,
    /// instead of checking for that
    /// TODO: investigate this further
    pub fn clear_ep(&mut self) {
        self.pos.key ^= Zobrist::ep(self.ep_file());
        self.pos.ep_square = 64;
    }

    pub fn add_piece(&mut self, side: Player, piece: PieceType, sq: Square) {
        assert!(piece != PieceType::None);

        self.pos.key ^= Zobrist::piece(side, piece, sq);
        unsafe {
            *self.pieces.get_unchecked_mut(sq as usize) = Piece::new(piece, side);

            let piece_bb = self.piece_bb.get_unchecked_mut(piece.as_usize());
            let side_bb = self.side_bb.get_unchecked_mut(side.as_usize());

            BitBoard::set_bit(piece_bb, sq);
            BitBoard::set_bit(side_bb, sq);
        }
    }

    pub fn remove_piece(&mut self, side: Player, piece: PieceType, sq: Square) {
        assert!(piece != PieceType::None);

        self.pos.key ^= Zobrist::piece(side, piece, sq);

        unsafe {
            *self.pieces.get_unchecked_mut(sq as usize) = Piece::NONE;
            let piece_bb = self.piece_bb.get_unchecked_mut(piece.as_usize());
            let side_bb = self.side_bb.get_unchecked_mut(side.as_usize());

            BitBoard::pop_bit(piece_bb, sq);
            BitBoard::pop_bit(side_bb, sq);
        }
    }

    pub fn debug(&mut self) {
        println!("{self:?}");

        let mut b = self.clone();
        while !b.history.empty() {
            let m = b.pos.last_move.unwrap();
            println!("{}", BitMove::pretty_move(m));
            if m == 0 {
                b.unmake_null_move();
            } else {
                b.unmake_move(m);
            }
            println!("{b:?}");
        }
    }
}

impl Board {
    pub const fn new() -> Self {
        Board {
            turn: Player::White,
            piece_bb: [BitBoard::EMPTY; NUM_PIECES],
            side_bb: [BitBoard::EMPTY; NUM_SIDES],
            pieces: [Piece::NONE; 64],
            pos: Position::new(),
            history: History::new(),
            killers: [[0; MAX_STACK_SIZE]; 2],
        }
    }

    /* #[allow(invalid_value)]
    /// Returns a mutable reference to an uninitialized board structure
    /// Not true anymore, this somehow only works in release mode
    /// temporary fix is just returning a new board
    pub fn uninit() -> Self {
        // unsafe { &mut *std::mem::MaybeUninit::<Board>::uninit().as_mut_ptr() }
        unsafe { *std::mem::MaybeUninit::<Board>::uninit().as_mut_ptr() }
    } */

    pub fn start_pos() -> Board {
        Board::from_fen(FEN_START_STRING)
    }

    pub fn from_fen(fen: &str) -> Board {
        let mut board = Board::new();

        let sections: Vec<&str> = fen.split_whitespace().collect();
        assert!(sections.len() == 6, "Invalid FEN string");

        let pieces_str = sections[0];
        let turn_str = sections[1];
        let castle_str = sections[2];
        let ep_str = sections[3];
        let half_move_str = sections[4];
        let full_move_str = sections[5];

        // Turn to move
        board.turn = match turn_str {
            "w" => Player::White,
            "b" => Player::Black,
            _ => panic!(),
        };

        // Castling permissions
        if !castle_str.contains('-') {
            for symbol in castle_str.split("") {
                if symbol.is_empty() {
                    continue;
                }
                board.pos.castling |= match symbol {
                    "K" => Castling::WK,
                    "Q" => Castling::WQ,
                    "k" => Castling::BK,
                    "q" => Castling::BQ,
                    _ => panic!("Invalid castling values in FEN string"),
                }
            }
        }

        // EP-square
        if !ep_str.contains('-') {
            board.set_ep(square_from_string(ep_str));
        }

        board.pos.half_move_count = half_move_str.parse::<u8>().unwrap();
        //board.pos.ply = full_move_str.parse::<usize>().unwrap();

        let mut file = 0;
        let mut rank = 7;

        // Piece locations
        for symbol in pieces_str.split("") {
            if symbol.is_empty() {
                continue;
            }
            let c: &str = &symbol.to_lowercase();
            let side = if c != symbol {
                Player::White
            } else {
                Player::Black
            };

            if c == "/" {
                file = 0;
                rank -= 1;
                continue;
            }
            if ["1", "2", "3", "4", "5", "6", "7", "8"].contains(&c) {
                file += c.parse::<Square>().unwrap();
                continue;
            }

            let square = rank * 8 + file;
            let piece = match c {
                "p" => PieceType::Pawn,
                "n" => PieceType::Knight,
                "b" => PieceType::Bishop,
                "r" => PieceType::Rook,
                "q" => PieceType::Queen,
                "k" => PieceType::King,
                _ => panic!(),
            };

            board.add_piece(side, piece, square);
            file += 1;
        }

        board.set_check_info();
        board.pos.key ^= Zobrist::castle(board.pos.castling);

        if board.turn == Player::Black {
            board.pos.key ^= Zobrist::side();
        }

        board
    }

    pub fn pretty_string(&self) -> String {
        let mut output = String::from("\n");

        for y in 0..8 {
            output.push_str("+---+---+---+---+---+---+---+---+\n");
            for x in 0..8 {
                let square = 8 * (7 - y) + x;
                let is_white =
                    BitBoard::from_sq(square) & self.side_bb[Player::White.as_usize()] != 0;

                let piece_str = match self.piece_type(square) {
                    PieceType::Pawn => " p ",
                    PieceType::Knight => " n ",
                    PieceType::Bishop => " b ",
                    PieceType::Rook => " r ",
                    PieceType::Queen => " q ",
                    PieceType::King => " k ",
                    PieceType::None => "   ",
                };

                output.push('|');
                if is_white {
                    output.push_str(&piece_str.to_uppercase());
                } else {
                    output.push_str(piece_str);
                }

                if x == 7 {
                    output.push('|');
                    output.push_str(&format!(" {}", (8 - y)));
                    output.push('\n');
                }
            }
        }
        output.push_str("+---+---+---+---+---+---+---+---+\n");
        output.push_str("  a   b   c   d   e   f   g   h  \n\n");

        output
    }
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pretty_string())
    }
}

impl std::fmt::Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pretty_string())?;
        writeln!(
            f,
            "Turn       : {}",
            match self.turn {
                Player::White => "White",
                Player::Black => "Black",
            }
        )?;
        writeln!(f, "Ply        : {}", self.pos.ply)?;
        writeln!(f, "Key        : {}", self.pos.key)?;
        writeln!(f, "Castling   : {:b}", self.pos.castling)?;
        writeln!(f, "EP Square  : {}", square_to_string(self.pos.ep_square))?;
        write!(f, "Checkers   : ")?;
        let mut checkers = self.pos.checkers_bb;
        while checkers != 0 {
            let checker_sq = BitBoard::pop_lsb(&mut checkers);
            write!(f, "{} ", square_to_string(checker_sq))?;
        }
        writeln!(f)?;
        writeln!(
            f,
            "Killer 1   : {}",
            BitMove::pretty_move(self.killers[0][self.pos.ply])
        )?;
        writeln!(
            f,
            "Killer 2   : {}",
            BitMove::pretty_move(self.killers[1][self.pos.ply])
        )?;

        writeln!(f)
    }
}
