use crate::{
    bitboard::BitBoard,
    bitmove::{BitMove, MoveFlag},
    board::Board,
    defs::{GenType, Piece, Player, Square},
    gen::{
        attack::{
            attacks, bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks,
        },
        between::between,
        eval::MVV_LVA,
    },
    movelist::MoveList,
    utils::adjacent_files,
};

const CAPTURE_BONUS: i32 = 1_000_000;
const KILLER_1_BONUS: i32 = 900_000;
const KILLER_2_BONUS: i32 = 800_000;

/// Bitboard of all the pieces that are attacking `square`
#[inline]
pub const fn attackers_to(board: &Board, sq: Square, occupied: u64) -> u64 {
    pawn_attacks(sq, Player::White) & board.player_piece_bb(Player::Black, Piece::Pawn)
        | pawn_attacks(sq, Player::Black) & board.player_piece_bb(Player::White, Piece::Pawn)
        | knight_attacks(sq) & board.piece_bb(Piece::Knight)
        | bishop_attacks(sq, occupied) & board.piece_like_bb(Piece::Bishop)
        | rook_attacks(sq, occupied) & board.piece_like_bb(Piece::Rook)
        | king_attacks(sq) & board.piece_bb(Piece::King)
}

pub const fn is_square_attacked(
    board: &Board,
    sq: Square,
    attacker_bb: u64,
    occupied: u64,
) -> bool {
    attackers_to(board, sq, occupied) & attacker_bb != 0
}

/// Compresses a bitboard of moves to u16 moves and adds them to the movelist.
/// Only supports sliding and knight moves
fn make_moves(
    board: &Board,
    src: Square,
    mut moves_bb: u64,
    opp_bb: u64,
    move_list: &mut MoveList,
) {
    while moves_bb != 0 {
        let dest = BitBoard::pop_lsb(&mut moves_bb);
        if BitBoard::contains(opp_bb, dest) {
            add_capture_move(
                BitMove::from_flag(src, dest, MoveFlag::CAPTURE),
                move_list,
                board,
            );
        } else {
            add_quiet_move(BitMove::from_squares(src, dest), move_list, board);
        }
    }
}

fn make_pawn_move(board: &Board, src: Square, dest: Square, flag: u8, move_list: &mut MoveList) {
    let m = BitMove::from_flag(src, dest, flag);

    if flag == MoveFlag::EN_PASSANT {
        add_ep_move(m, move_list);
    } else if flag & MoveFlag::CAPTURE != 0 {
        add_capture_move(m, move_list, board);
    } else {
        add_quiet_move(m, move_list, board)
    }
}

fn add_quiet_move(m: u16, move_list: &mut MoveList, board: &Board) {
    assert!(!BitMove::is_cap(m));

    let mut score = 0;
    let ply = board.pos.ply;
    if board.killers[0][ply] == m {
        score = KILLER_1_BONUS;
    } else if board.killers[1][ply] == m {
        score = KILLER_2_BONUS;
    }

    move_list.push(m, score);
}

fn add_capture_move(m: u16, move_list: &mut MoveList, board: &Board) {
    let move_piece = board.piece(BitMove::src(m));
    let cap_piece = board.piece(BitMove::dest(m));

    assert!(move_piece != Piece::None);
    assert!(cap_piece != Piece::None);

    let score = MVV_LVA[move_piece.as_usize()][cap_piece.as_usize()] + CAPTURE_BONUS;
    move_list.push(m, score);
}

fn add_ep_move(m: u16, move_list: &mut MoveList) {
    move_list.push(m, 105 + CAPTURE_BONUS);
}

fn make_promotions(
    board: &Board,
    src: Square,
    dest: Square,
    gen_type: &GenType,
    capture: bool,
    move_list: &mut MoveList,
) {
    let flag = if capture { MoveFlag::CAPTURE } else { 0 };

    if gen_type == &GenType::Captures
        || gen_type == &GenType::EvadingCaptures
        || gen_type == &GenType::Evasions
        || gen_type == &GenType::NonEvasions
    {
        make_pawn_move(board, src, dest, MoveFlag::PROMOTE_QUEEN | flag, move_list);
    }
    if gen_type == &GenType::Quiets
        || gen_type == &GenType::Evasions
        || gen_type == &GenType::NonEvasions
    {
        make_pawn_move(board, src, dest, MoveFlag::PROMOTE_KNIGHT | flag, move_list);
        make_pawn_move(board, src, dest, MoveFlag::PROMOTE_BISHOP | flag, move_list);
        make_pawn_move(board, src, dest, MoveFlag::PROMOTE_ROOK | flag, move_list);
    }
}

#[inline]
const fn pawn_push(pawns: u64, player: Player) -> u64 {
    match player {
        Player::White => pawns << 8,
        Player::Black => pawns >> 8,
    }
}

#[inline]
const fn double_pawn_push(pawns: u64, player: Player) -> u64 {
    match player {
        Player::White => pawns << 16,
        Player::Black => pawns >> 16,
    }
}

#[inline]
const fn pawn_cap_east(pawns: u64, player: Player) -> u64 {
    match player {
        Player::White => (pawns & !BitBoard::FILE_H) << 9,
        Player::Black => (pawns & !BitBoard::FILE_H) >> 7,
    }
}

#[inline]
const fn pawn_cap_west(pawns: u64, player: Player) -> u64 {
    match player {
        Player::White => (pawns & !BitBoard::FILE_A) << 7,
        Player::Black => (pawns & !BitBoard::FILE_A) >> 9,
    }
}

fn gen_pawn_moves(board: &Board, target: u64, gen_type: &GenType, move_list: &mut MoveList) {
    let opp = board.turn.opp();
    let opp_bb = board.player_bb(opp);
    let occ = board.occ_bb();
    let opp_king_sq = board.king_square(opp);
    let pawn_dir = board.turn.pawn_dir();
    let rank_3 = board.turn.rank_3();
    let rank_7 = board.turn.rank_7();

    let pawns = board.player_piece_bb(board.turn, Piece::Pawn);
    let pwn_rank_7 = pawns & rank_7;
    let pwn_not_rank_7 = pawns & !rank_7;

    // Quiet pushes
    if gen_type != &GenType::Captures && gen_type != &GenType::EvadingCaptures {
        // One square forward
        let mut m1 = pawn_push(pwn_not_rank_7, board.turn) & !occ;
        // Double pawn push
        let mut m2 = pawn_push(m1 & rank_3, board.turn) & !occ;

        if gen_type == &GenType::Evasions || gen_type == &GenType::EvadingCaptures {
            m1 &= target;
            m2 &= target;
        } else if gen_type == &GenType::QuietChecks {
            let blockers = board.blockers(opp) & pwn_not_rank_7 & !BitBoard::file_bb(opp_king_sq);
            let atk = pawn_attacks(opp_king_sq, opp);

            // Direct check or move a blocker
            m1 &= atk | pawn_push(blockers, board.turn);
            m2 &= atk | double_pawn_push(blockers, board.turn);
        }

        while m1 != 0 {
            let dest = BitBoard::pop_lsb(&mut m1);
            let src = dest - pawn_dir;
            let flag = MoveFlag::QUIET;
            make_pawn_move(board, src, dest, flag, move_list);
        }
        while m2 != 0 {
            let dest = BitBoard::pop_lsb(&mut m2);
            let src = dest - pawn_dir - pawn_dir;
            let flag = MoveFlag::DOUBLE_PAWN_PUSH;
            make_pawn_move(board, src, dest, flag, move_list);
        }
    }

    // Captures
    if gen_type != &GenType::Quiets && gen_type != &GenType::QuietChecks {
        // Capture east of pawn
        let mut m1 = pawn_cap_east(pwn_not_rank_7, board.turn) & opp_bb;
        // Capture west of pawn
        let mut m2 = pawn_cap_west(pwn_not_rank_7, board.turn) & opp_bb;

        if gen_type == &GenType::Evasions || gen_type == &GenType::EvadingCaptures {
            m1 &= board.pos.checkers_bb;
            m2 &= board.pos.checkers_bb;
        }

        while m1 != 0 {
            let dest = BitBoard::pop_lsb(&mut m1);
            let flag = MoveFlag::CAPTURE;
            make_pawn_move(board, dest - pawn_dir - 1, dest, flag, move_list);
        }
        while m2 != 0 {
            let dest = BitBoard::pop_lsb(&mut m2);
            let flag = MoveFlag::CAPTURE;
            make_pawn_move(board, dest - pawn_dir + 1, dest, flag, move_list);
        }

        if board.can_ep() {
            let ep_file = board.ep_file();
            let ep_pawn_sq = board.pos.ep_square - pawn_dir;

            let mut m3 = pwn_not_rank_7 & BitBoard::rank_bb(ep_pawn_sq) & adjacent_files(ep_file);

            while m3 != 0 {
                let src = BitBoard::pop_lsb(&mut m3);
                let flag = MoveFlag::EN_PASSANT;
                make_pawn_move(board, src, board.pos.ep_square, flag, move_list);
            }
        }
    }

    // Promotions
    if pwn_rank_7 != 0 && gen_type != &GenType::QuietChecks {
        // Promotion by normal pawn push
        let mut m1 = pawn_push(pwn_rank_7, board.turn) & !occ;
        // Promotion by capturing east of pawn
        let mut m2 = pawn_cap_east(pwn_rank_7, board.turn) & opp_bb;
        // Promotion by capturing west of pawn
        let mut m3 = pawn_cap_west(pwn_rank_7, board.turn) & opp_bb;

        if gen_type == &GenType::Evasions || gen_type == &GenType::EvadingCaptures {
            m1 &= target;
            m2 &= target;
            m3 &= target;
        }

        while m1 != 0 {
            let dest = BitBoard::pop_lsb(&mut m1);
            let src = dest - pawn_dir;
            make_promotions(board, src, dest, gen_type, false, move_list);
        }

        while m2 != 0 {
            let dest = BitBoard::pop_lsb(&mut m2);
            let src = dest - pawn_dir - 1;
            make_promotions(board, src, dest, gen_type, true, move_list);
        }
        while m3 != 0 {
            let dest = BitBoard::pop_lsb(&mut m3);
            let src = dest - pawn_dir + 1;
            make_promotions(board, src, dest, gen_type, true, move_list);
        }
    }
}

// Generate sliding and knight moves
fn gen_piece_moves(
    board: &Board,
    piece: Piece,
    target: u64,
    checks: bool,
    move_list: &mut MoveList,
) {
    let mut piece_bb = board.player_piece_bb(board.turn, piece);
    while piece_bb != 0 {
        let sq = BitBoard::pop_lsb(&mut piece_bb);
        let mut bb = attacks(piece, sq, board.occ_bb(), board.turn) & target;

        if checks {
            // Moving a blocker also causes check
            if piece == Piece::Queen
                || board.pos.king_blockers[board.turn.opp().as_usize()] & BitBoard::from_sq(sq) == 0
            {
                bb &= board.pos.check_squares[piece.as_usize()];
            }
        }

        make_moves(board, sq, bb, board.player_bb(board.turn.opp()), move_list);
    }
}

fn generate_all(board: &Board, gen_type: GenType, move_list: &mut MoveList) {
    let king_sq = board.cur_king_square();
    let checker_sq = BitBoard::bit_scan_forward(board.pos.checkers_bb);
    let checks = gen_type == GenType::QuietChecks;
    let mut target_bb = BitBoard::EMPTY;

    // Don' t generate piece moves in double check
    if (gen_type != GenType::Evasions && gen_type != GenType::EvadingCaptures)
        || !BitBoard::more_than_one(board.pos.checkers_bb)
    {
        target_bb = match gen_type {
            GenType::Evasions => between(king_sq, checker_sq) | BitBoard::from_sq(checker_sq),
            GenType::EvadingCaptures => {
                (between(king_sq, checker_sq) | BitBoard::from_sq(checker_sq))
                    & board.player_bb(board.turn.opp())
            }
            GenType::NonEvasions => !board.cur_player_bb(),
            GenType::Captures => board.player_bb(board.turn.opp()),
            GenType::Quiets | GenType::QuietChecks => !board.occ_bb(),
        };

        gen_pawn_moves(board, target_bb, &gen_type, move_list);
        gen_piece_moves(board, Piece::Knight, target_bb, checks, move_list);
        gen_piece_moves(board, Piece::Bishop, target_bb, checks, move_list);
        gen_piece_moves(board, Piece::Rook, target_bb, checks, move_list);
        gen_piece_moves(board, Piece::Queen, target_bb, checks, move_list);
    }

    if !checks || BitBoard::contains(board.blockers(board.turn.opp()), king_sq) {
        let mut bb = king_attacks(king_sq);
        if gen_type == GenType::Evasions {
            bb &= !board.cur_player_bb();
        } else {
            bb &= target_bb;
        }

        if checks {
            // Check by moving a blocked piece
            let opp_king_sq = board.king_square(board.turn.opp());
            bb &= !between(king_sq, opp_king_sq);
        }

        let opp_bb = board.player_bb(board.turn.opp());
        while bb != 0 {
            let dest = BitBoard::pop_lsb(&mut bb);
            if BitBoard::contains(opp_bb, dest) {
                add_capture_move(
                    BitMove::from_flag(king_sq, dest, MoveFlag::CAPTURE),
                    move_list,
                    board,
                );
            } else {
                add_quiet_move(BitMove::from_squares(king_sq, dest), move_list, board);
            }
        }

        // Castling
        if (gen_type == GenType::Quiets || gen_type == GenType::NonEvasions)
            && !board.in_check()
            && board.can_castle(board.turn)
        {
            let occ = board.occ_bb();
            if board.can_castle_king(board.turn)
                && !BitBoard::contains(occ, king_sq + 1)
                && !BitBoard::contains(occ, king_sq + 2)
            {
                add_quiet_move(
                    BitMove::from_flag(king_sq, king_sq + 2, MoveFlag::CASTLE_KING),
                    move_list,
                    board,
                );
            }
            if board.can_castle_queen(board.turn)
                && !BitBoard::contains(occ, king_sq - 1)
                && !BitBoard::contains(occ, king_sq - 2)
                && !BitBoard::contains(occ, king_sq - 3)
            {
                add_quiet_move(
                    BitMove::from_flag(king_sq, king_sq - 2, MoveFlag::CASTLE_QUEEN),
                    move_list,
                    board,
                );
            }
        }
    }
}

/// Wrapper around [`generate_all`]
pub fn generate_legal(board: &mut Board, move_list: &mut MoveList) {
    let mut pseudo = MoveList::new();

    if board.in_check() {
        generate_all(board, GenType::Evasions, &mut pseudo);
    } else {
        generate_all(board, GenType::NonEvasions, &mut pseudo);
    }

    let mut i = 0;
    while i < pseudo.size() {
        let (m, score) = pseudo.get_all(i);
        if is_legal_move(board, m) {
            move_list.push(m, score);
        }
        i += 1;
    }
}

pub fn generate_quiet(board: &mut Board, move_list: &mut MoveList) {
    let mut pseudo = MoveList::new();

    if board.in_check() {
        generate_all(board, GenType::EvadingCaptures, &mut pseudo);
    } else {
        generate_all(board, GenType::Captures, &mut pseudo);
        generate_all(board, GenType::QuietChecks, &mut pseudo);
    }

    let mut i = 0;
    while i < pseudo.size() {
        let (m, score) = pseudo.get_all(i);
        if is_legal_move(board, m) {
            move_list.push(m, score);
        }
        i += 1;
    }
}

const fn is_legal_move(board: &Board, m: u16) -> bool {
    let blockers = board.blockers(board.turn);
    let flag = BitMove::flag(m);
    let src = BitMove::src(m);
    let dest = BitMove::dest(m);
    let king_sq = board.cur_king_square();

    if flag == MoveFlag::CASTLE_KING || flag == MoveFlag::CASTLE_QUEEN {
        // Can't castle when in check
        if board.in_check() {
            return false;
        }

        // Between squares can't be attacked
        let opp_bb = board.player_bb(board.turn.opp());
        let occ = board.occ_bb();
        let dir = if flag == MoveFlag::CASTLE_KING { 1 } else { -1 };

        if is_square_attacked(board, king_sq + dir, opp_bb, occ) {
            return false;
        }
        if is_square_attacked(board, king_sq + dir + dir, opp_bb, occ) {
            return false;
        }

        return true;
    }

    if king_sq == src {
        return !is_square_attacked(
            board,
            dest,
            board.player_bb(board.turn.opp()),
            board.occ_bb() ^ BitBoard::from_sq(src),
        );
    }

    match flag {
        MoveFlag::EN_PASSANT => {
            let cap_sq = board.pos.ep_square - board.turn.pawn_dir();
            let occ = board.occ_bb() ^ BitBoard::from_sq(src) ^ BitBoard::from_sq(cap_sq)
                | BitBoard::from_sq(dest);
            let bishop_like_bb = board.player_piece_like_bb(board.turn.opp(), Piece::Bishop);
            let rook_like_bb = board.player_piece_like_bb(board.turn.opp(), Piece::Rook);

            bishop_attacks(king_sq, occ) & bishop_like_bb == 0
                && rook_attacks(king_sq, occ) & rook_like_bb == 0
        }
        _ => {
            return !BitBoard::contains(blockers, src)
                || BitBoard::triple_aligned(src, dest, king_sq);
        }
    }
}

pub const fn smallest_attacker(board: &Board, sq: Square, side: Player) -> (Piece, Square) {
    let pawns = pawn_attacks(sq, side) & board.player_piece_bb(side, Piece::Pawn);
    if pawns != 0 {
        return (Piece::Pawn, BitBoard::bit_scan_forward(pawns));
    }
    let knights = knight_attacks(sq) & board.player_piece_bb(side, Piece::Knight);
    if knights != 0 {
        return (Piece::Knight, BitBoard::bit_scan_forward(knights));
    }

    let occ = board.occ_bb();

    let bishop_moves = bishop_attacks(sq, occ);
    let bishops = bishop_moves & board.player_piece_bb(side, Piece::Bishop);
    if bishops != 0 {
        return (Piece::Bishop, BitBoard::bit_scan_forward(bishops));
    }

    let rook_moves = rook_attacks(sq, occ);
    let rooks = rook_moves & board.player_piece_bb(side, Piece::Rook);
    if rooks != 0 {
        return (Piece::Rook, BitBoard::bit_scan_forward(rooks));
    }

    let queens = (bishop_moves | rook_moves) & board.player_piece_bb(side, Piece::Queen);
    if queens != 0 {
        return (Piece::Queen, BitBoard::bit_scan_forward(queens));
    }

    let king = king_attacks(sq) & board.player_piece_bb(side, Piece::King);
    if king != 0 {
        return (Piece::King, BitBoard::bit_scan_forward(king));
    }

    (Piece::None, 64)
}
