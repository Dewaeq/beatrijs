use std::cmp;

use crate::{
    bitboard::BitBoard,
    bitmove::{BitMove, MoveFlag},
    defs::{
        Castling, Piece, PieceType, Player, Score, Square, BLACK_IDX, FEN_START_STRING,
        MAX_MOVES, NUM_PIECES, NUM_SIDES, NUM_SQUARES, WHITE_IDX, MG_VALUE,
    },
    gen::{attack::attacks, between::between},
    history::History,
    movegen::{attackers_to, smallest_attacker},
    position::Position,
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
    pub killers: [[u16; MAX_MOVES]; 2],
}

/// Getter methods
impl Board {
    pub const fn key(&self) -> u64 {
        self.pos.key
    }

    /// Get the [`PieceType`] of the piece on the provided square
    pub const fn piece(&self, square: Square) -> PieceType {
        unsafe { self.pieces.get_unchecked(square as usize).t }
    }

    pub const fn occ_bb(&self) -> u64 {
        unsafe { *self.side_bb.get_unchecked(0) | *self.side_bb.get_unchecked(1) }
    }

    pub const fn cur_player_bb(&self) -> u64 {
        self.player_bb(self.turn)
    }

    pub const fn player_bb(&self, side: Player) -> u64 {
        unsafe {
            match side {
                Player::White => *self.side_bb.get_unchecked(WHITE_IDX),
                _ => *self.side_bb.get_unchecked(BLACK_IDX),
            }
        }
    }

    pub const fn piece_bb(&self, piece: PieceType) -> u64 {
        unsafe { *self.piece_bb.get_unchecked(piece.as_usize()) }
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

    pub const fn has_big_piece(&self, side: Player) -> bool {
        self.player_piece_bb(side, PieceType::Bishop) != 0
            || self.player_piece_bb(side, PieceType::Rook) != 0
            || self.player_piece_bb(side, PieceType::Queen) != 0
    }

    pub const fn blockers(&self, side: Player) -> u64 {
        unsafe { *self.pos.king_blockers.get_unchecked(side.as_usize()) }
    }

    pub fn slider_blockers(&self, sq: Square, us_bb: u64, opp_bb: u64) -> u64 {
        let opp = self.turn.opp();
        // Every piece of the opponent which is a possible pinner
        let mut pinners = attacks(PieceType::Bishop, sq, opp_bb, opp)
            & self.player_piece_like_bb(opp, PieceType::Bishop)
            | attacks(PieceType::Rook, sq, opp_bb, opp)
                & self.player_piece_like_bb(opp, PieceType::Rook);
        let mut pinned_bb = BitBoard::EMPTY;

        while pinners != 0 {
            let pinner_sq = BitBoard::pop_lsb(&mut pinners);
            // Possibly pinned pieces
            let possibly_pinned_bb = between(sq, pinner_sq) & us_bb;
            if !BitBoard::more_than_one(possibly_pinned_bb) {
                pinned_bb |= possibly_pinned_bb;
            }
        }

        pinned_bb
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

        if king_sq >= 64 {
            self.debug();
        }

        assert!(king_sq < 64);
        assert!(opp_king_sq < 64);

        // Reset checkers and pinners
        self.pos.checkers_bb = 0;
        self.pos.king_blockers = [0, 0];

        self.pos.checkers_bb = attackers_to(self, king_sq, occ) & self.player_bb(opp);

        unsafe {
            *self
                .pos
                .king_blockers
                .get_unchecked_mut(self.turn.as_usize()) =
                self.slider_blockers(king_sq, us_bb, opp_bb);
            *self.pos.king_blockers.get_unchecked_mut(opp.as_usize()) =
                self.slider_blockers(opp_king_sq, us_bb, opp_bb);

            self.set_check_squares(
                PieceType::Pawn,
                attacks(PieceType::Pawn, opp_king_sq, 0, self.turn),
            );
            self.set_check_squares(
                PieceType::Knight,
                attacks(PieceType::Knight, opp_king_sq, 0, self.turn),
            );
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
                self.pos
                    .check_squares
                    .get_unchecked(PieceType::Bishop.as_usize())
                    | self
                        .pos
                        .check_squares
                        .get_unchecked(PieceType::Rook.as_usize()),
            );
        }
    }

    fn set_check_squares(&mut self, piece: PieceType, bb: u64) {
        unsafe { *self.pos.check_squares.get_unchecked_mut(piece.as_usize()) = bb }
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
        let piece = self.piece(src);
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
            let cap_pt = self.piece(dest);
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
            self.pos.rule_fifty = 0;
        } else {
            self.pos.rule_fifty += 1;
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
        let piece = self.piece(dest);
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
        self.killers = [[0; MAX_MOVES]; 2];
    }

    pub fn see_capture(&self, m: u16) -> Score {
        if !BitMove::is_cap(m) {
            return 0;
        }

        let captured = self.piece(BitMove::dest(m));
        let mut new_board: Board = *self;
        new_board.make_move(m);

        MG_VALUE[captured.as_usize()] - new_board.see(BitMove::dest(m))
    }

    fn see(&mut self, dest: Square) -> Score {
        let captured = self.piece(dest);
        let (attacker, src) = smallest_attacker(self, dest, self.turn);

        if attacker != PieceType::None {
            self.move_piece_cheap(src, dest, attacker, captured);
            cmp::max(0, MG_VALUE[captured.as_usize()] - self.see(dest))
        } else {
            0
        }
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
            killers: [[0; MAX_MOVES]; 2],
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

        board.pos.rule_fifty = half_move_str.parse::<u8>().unwrap();
        board.pos.ply = full_move_str.parse::<usize>().unwrap();

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

                let piece_str = match self.piece(square) {
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

        writeln!(f)
    }
}
