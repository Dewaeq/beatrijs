use crate::{
    bitboard::BitBoard,
    bitmove::{BitMove, MoveFlag},
    defs::{
        Castling, PieceType, Player, Square, BLACK_IDX, FEN_START_STRING, NUM_PIECES, NUM_SIDES,
        NUM_SQUARES, WHITE_IDX,
    },
    gen::{attack::attacks, between::between},
    movegen::attackers_to,
    position::Position,
    utils::square_from_string,
    zobrist::Zobrist,
};

#[derive(Clone, Copy)]
pub struct Board {
    pub turn: Player,
    pub piece_bb: [u64; NUM_PIECES],
    pub side_bb: [u64; NUM_SIDES],
    pub pieces: [PieceType; NUM_SQUARES],
    pub pos: Position,
}

/// Getter methods
impl Board {
    /// Get the [`PieceType`] of the piece on the provided square
    pub const fn piece_type(&self, square: Square) -> PieceType {
        self.pieces[square as usize]
    }

    pub const fn occ_bb(&self) -> u64 {
        self.side_bb[0] | self.side_bb[1]
    }

    pub const fn cur_player_bb(&self) -> u64 {
        self.player_bb(self.turn)
    }

    pub const fn player_bb(&self, side: Player) -> u64 {
        match side {
            Player::White => self.side_bb[WHITE_IDX],
            _ => self.side_bb[BLACK_IDX],
        }
    }

    pub const fn piece_bb(&self, piece_type: PieceType) -> u64 {
        self.piece_bb[piece_type.as_usize()]
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

    pub const fn player_piece_bb(&self, side: Player, piece_type: PieceType) -> u64 {
        let piece_bb = self.piece_bb(piece_type);
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

    pub const fn blockers(&self, side: Player) -> u64 {
        self.pos.king_blockers[side.as_usize()]
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

        // Reset checkers and pinners
        self.pos.checkers_bb = 0;
        self.pos.king_blockers = [0, 0];

        self.pos.checkers_bb = attackers_to(self, king_sq, occ) & self.player_bb(opp);

        self.pos.king_blockers[self.turn.as_usize()] = self.slider_blockers(king_sq, us_bb, opp_bb);
        self.pos.king_blockers[opp.as_usize()] = self.slider_blockers(opp_king_sq, opp_bb, us_bb);

        self.pos.check_squares[PieceType::Pawn.as_usize()] =
            attacks(PieceType::Pawn, opp_king_sq, 0, self.turn);
        self.pos.check_squares[PieceType::Knight.as_usize()] =
            attacks(PieceType::Knight, opp_king_sq, 0, self.turn);
        self.pos.check_squares[PieceType::Bishop.as_usize()] =
            attacks(PieceType::Bishop, opp_king_sq, occ, self.turn);
        self.pos.check_squares[PieceType::Rook.as_usize()] =
            attacks(PieceType::Rook, opp_king_sq, occ, self.turn);
        self.pos.check_squares[PieceType::Queen.as_usize()] = self.pos.check_squares
            [PieceType::Bishop.as_usize()]
            | self.pos.check_squares[PieceType::Rook.as_usize()];
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
        let piece_type = self.pieces[src as usize];
        let opp = self.turn.opp();

        assert!(piece_type != PieceType::None);

        // Remove all castling rights for the moving side when a king move occurs
        if piece_type == PieceType::King {
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
            let prom_type = BitMove::prom_piece_type(flag);
            self.add_piece(self.turn, prom_type, dest);
            // target.pos.key ^= Zobrist::piece(self.turn, prom_type, dest);
        } else {
            self.add_piece(self.turn, piece_type, dest);
            // target.pos.key ^= Zobrist::piece(self.turn, piece_type, dest);
        }

        if self.pos.castling != self.pos.castling {
            self.pos.key ^= Zobrist::castle(self.pos.castling);
        }

        self.pos.key ^= Zobrist::side();
        // target.pos.key ^= Zobrist::piece(self.turn, piece_type, src);

        self.remove_piece(self.turn, piece_type, src);
        self.set_castling_from_move(m);
        self.turn = self.turn.opp();
        self.pos.ply += 1;
        self.set_check_info();
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

    pub fn add_piece(&mut self, side: Player, piece_type: PieceType, sq: Square) {
        assert!(piece_type != PieceType::None);

        self.pos.key ^= Zobrist::piece(side, piece_type, sq);
        self.pieces[sq as usize] = piece_type;

        let piece_bb = &mut self.piece_bb[piece_type.as_usize()];
        let side_bb = match side {
            Player::White => &mut self.side_bb[WHITE_IDX],
            _ => &mut self.side_bb[BLACK_IDX],
        };

        BitBoard::set_bit(piece_bb, sq);
        BitBoard::set_bit(side_bb, sq);
    }

    pub fn remove_piece(&mut self, side: Player, piece_type: PieceType, sq: Square) {
        assert!(piece_type != PieceType::None);

        self.pieces[sq as usize] = PieceType::None;
        self.pos.key ^= Zobrist::piece(side, piece_type, sq);

        let piece_bb = &mut self.piece_bb[piece_type.as_usize()];
        let side_bb = match side {
            Player::White => &mut self.side_bb[WHITE_IDX],
            _ => &mut self.side_bb[BLACK_IDX],
        };

        BitBoard::pop_bit(piece_bb, sq);
        BitBoard::pop_bit(side_bb, sq);
    }
}

impl Board {
    pub const fn new() -> Self {
        Board {
            turn: Player::White,
            piece_bb: [BitBoard::EMPTY; NUM_PIECES],
            side_bb: [BitBoard::EMPTY; NUM_SIDES],
            pieces: [PieceType::None; 64],
            pos: Position::new(),
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
        if !castle_str.contains("-") {
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
        if !ep_str.contains("-") {
            board.set_ep(square_from_string(ep_str));
        }

        board.pos.rule_fifty = half_move_str.parse::<u8>().unwrap();
        board.pos.ply = full_move_str.parse::<u16>().unwrap();

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
            let piece_type = match c {
                "p" => PieceType::Pawn,
                "n" => PieceType::Knight,
                "b" => PieceType::Bishop,
                "r" => PieceType::Rook,
                "q" => PieceType::Queen,
                "k" => PieceType::King,
                _ => panic!(),
            };

            board.add_piece(side, piece_type, square);
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
                let is_white = BitBoard::from_sq(square) & self.side_bb[Player::White] != 0;
                // let piece_str = self.pieces[square as usize].to_string();

                let piece_str = match self.pieces[square as usize] {
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
                    output.push_str(&format!(" {}", &(8 - y).to_string()));
                    output.push_str("\n");
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
        writeln!(f, "EP Square  : {}", self.pos.ep_square)
    }
}
