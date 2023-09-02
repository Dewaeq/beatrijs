use crate::{
    bitboard::BitBoard,
    bitmove::{BitMove, MoveFlag},
    board::Board,
    defs::{GenType, PieceType, Player, Square},
    gen::{
        attack::{
            attacks, bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks,
        },
        between::between,
        eval::MVV_LVA,
    },
    movelist::MoveList,
    search::HistoryTable,
    utils::adjacent_files,
};

const HASH_BONUS: i32 = 9_000_000;
const PROMOTE_BONUS: i32 = 7_000_000;
const GOOD_CAPTURE_BONUS: i32 = 6_000_000;
const KILLER_1_BONUS: i32 = 5_000_000;
const KILLER_2_BONUS: i32 = 4_000_000;
const BAD_CAPTURE_BONUS: i32 = 3_000_000;

pub struct MovegenParams<'a> {
    board: &'a Board,
    history_table: &'a HistoryTable,
    hash_move: u16,
}

impl<'a> MovegenParams<'a> {
    pub fn new(board: &'a Board, history_table: &'a HistoryTable, hash_move: u16) -> Self {
        MovegenParams {
            board,
            history_table,
            hash_move,
        }
    }

    pub fn simple(board: &'a Board) -> Self {
        MovegenParams {
            board,
            history_table: &[[[0; 64]; 64]; 2],
            hash_move: 0,
        }
    }
}

/// Bitboard of all the pieces that are attacking `square`
#[inline]
pub const fn attackers_to(board: &Board, sq: Square, occupied: u64) -> u64 {
    pawn_attacks(sq, Player::White) & board.player_piece_bb(Player::Black, PieceType::Pawn)
        | pawn_attacks(sq, Player::Black) & board.player_piece_bb(Player::White, PieceType::Pawn)
        | knight_attacks(sq) & board.piece_bb(PieceType::Knight)
        | bishop_attacks(sq, occupied) & board.piece_like_bb(PieceType::Bishop)
        | rook_attacks(sq, occupied) & board.piece_like_bb(PieceType::Rook)
        | king_attacks(sq) & board.piece_bb(PieceType::King)
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
    params: &MovegenParams,
    src: Square,
    mut moves_bb: u64,
    opp_bb: u64,
    move_list: &mut MoveList,
) {
    while moves_bb != 0 {
        let dest = BitBoard::pop_lsb(&mut moves_bb);
        if BitBoard::contains(opp_bb, dest) {
            let m = BitMove::from_flag(src, dest, MoveFlag::CAPTURE);
            add_move(m, params, move_list);
        } else {
            let m = BitMove::from_squares(src, dest);
            add_move(m, params, move_list);
        }
    }
}

fn add_move(m: u16, params: &MovegenParams, move_list: &mut MoveList) {
    let score = score_move(m, params);
    move_list.push(m, score);
}

fn score_move(m: u16, params: &MovegenParams) -> i32 {
    let (src, dest) = (BitMove::src(m), BitMove::dest(m));

    if m == params.hash_move {
        HASH_BONUS
    } else if BitMove::is_prom(m) {
        PROMOTE_BONUS
    } else if BitMove::is_cap(m) {
        let mvv_lva = if BitMove::is_ep(m) {
            MVV_LVA[0][0]
        } else {
            let move_piece = params.board.piece_type(BitMove::src(m));
            let cap_piece = params.board.piece_type(BitMove::dest(m));
            MVV_LVA[move_piece.as_usize()][cap_piece.as_usize()]
        };

        if params.board.see_ge(m, 0) {
            GOOD_CAPTURE_BONUS + mvv_lva
        } else {
            BAD_CAPTURE_BONUS + mvv_lva
        }
    } else if m == params.board.killers[0][params.board.pos.ply] {
        KILLER_1_BONUS
    } else if m == params.board.killers[1][params.board.pos.ply] {
        KILLER_2_BONUS
    } else {
        params.history_table[params.board.turn.as_usize()][src as usize][dest as usize]
    }
}

fn make_promotions(
    params: &MovegenParams,
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
        let m = BitMove::from_flag(src, dest, MoveFlag::PROMOTE_QUEEN | flag);
        add_move(m, params, move_list);
    }
    if gen_type == &GenType::Quiets
        || gen_type == &GenType::Evasions
        || gen_type == &GenType::NonEvasions
    {
        let m = BitMove::from_flag(src, dest, MoveFlag::PROMOTE_KNIGHT | flag);
        add_move(m, params, move_list);

        let m = BitMove::from_flag(src, dest, MoveFlag::PROMOTE_BISHOP | flag);
        add_move(m, params, move_list);

        let m = BitMove::from_flag(src, dest, MoveFlag::PROMOTE_ROOK | flag);
        add_move(m, params, move_list);
    }
}

#[inline]
pub const fn pawn_push(pawns: u64, player: Player) -> u64 {
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

#[inline]
pub const fn pawn_caps(pawns: u64, player: Player) -> u64 {
    pawn_cap_west(pawns, player) | pawn_cap_east(pawns, player)
}

fn gen_pawn_moves(
    params: &MovegenParams,
    target: u64,
    gen_type: &GenType,
    move_list: &mut MoveList,
) {
    let opp = params.board.turn.opp();
    let opp_bb = params.board.player_bb(opp);
    let occ = params.board.occ_bb();
    let opp_king_sq = params.board.king_square(opp);
    let pawn_dir = params.board.turn.pawn_dir();
    let rank_3 = params.board.turn.rank_3();
    let rank_7 = params.board.turn.rank_7();

    let pawns = params
        .board
        .player_piece_bb(params.board.turn, PieceType::Pawn);
    let pwn_rank_7 = pawns & rank_7;
    let pwn_not_rank_7 = pawns & !rank_7;

    // Quiet pushes
    if gen_type != &GenType::Captures && gen_type != &GenType::EvadingCaptures {
        // One square forward
        let mut m1 = pawn_push(pwn_not_rank_7, params.board.turn) & !occ;
        // Double pawn push
        let mut m2 = pawn_push(m1 & rank_3, params.board.turn) & !occ;

        if gen_type == &GenType::Evasions || gen_type == &GenType::EvadingCaptures {
            m1 &= target;
            m2 &= target;
        } else if gen_type == &GenType::QuietChecks {
            let blockers =
                params.board.blockers(opp) & pwn_not_rank_7 & !BitBoard::file_bb(opp_king_sq);
            let atk = pawn_attacks(opp_king_sq, opp);

            // Direct check or move a blocker
            m1 &= atk | pawn_push(blockers, params.board.turn);
            m2 &= atk | double_pawn_push(blockers, params.board.turn);
        }

        while m1 != 0 {
            let dest = BitBoard::pop_lsb(&mut m1);
            let src = dest - pawn_dir;
            let flag = MoveFlag::QUIET;

            let m = BitMove::from_flag(src, dest, flag);
            add_move(m, params, move_list);
        }
        while m2 != 0 {
            let dest = BitBoard::pop_lsb(&mut m2);
            let src = dest - pawn_dir - pawn_dir;
            let flag = MoveFlag::DOUBLE_PAWN_PUSH;

            let m = BitMove::from_flag(src, dest, flag);
            add_move(m, params, move_list);
        }
    }

    // Captures
    if gen_type != &GenType::Quiets && gen_type != &GenType::QuietChecks {
        // Capture east of pawn
        let mut m1 = pawn_cap_east(pwn_not_rank_7, params.board.turn) & opp_bb;
        // Capture west of pawn
        let mut m2 = pawn_cap_west(pwn_not_rank_7, params.board.turn) & opp_bb;

        if gen_type == &GenType::Evasions || gen_type == &GenType::EvadingCaptures {
            m1 &= params.board.pos.checkers_bb;
            m2 &= params.board.pos.checkers_bb;
        }

        while m1 != 0 {
            let dest = BitBoard::pop_lsb(&mut m1);
            let flag = MoveFlag::CAPTURE;

            let m = BitMove::from_flag(dest - pawn_dir - 1, dest, flag);
            add_move(m, params, move_list);
        }
        while m2 != 0 {
            let dest = BitBoard::pop_lsb(&mut m2);
            let flag = MoveFlag::CAPTURE;

            let m = BitMove::from_flag(dest - pawn_dir + 1, dest, flag);
            add_move(m, params, move_list);
        }

        if params.board.can_ep() {
            let ep_file = params.board.ep_file();
            let ep_pawn_sq = params.board.pos.ep_square - pawn_dir;

            let mut m3 = pwn_not_rank_7 & BitBoard::rank_bb(ep_pawn_sq) & adjacent_files(ep_file);

            while m3 != 0 {
                let src = BitBoard::pop_lsb(&mut m3);
                let flag = MoveFlag::EN_PASSANT;

                let m = BitMove::from_flag(src, params.board.pos.ep_square, flag);
                add_move(m, params, move_list);
            }
        }
    }

    // Promotions
    if pwn_rank_7 != 0 && gen_type != &GenType::QuietChecks {
        // Promotion by normal pawn push
        let mut m1 = pawn_push(pwn_rank_7, params.board.turn) & !occ;
        // Promotion by capturing east of pawn
        let mut m2 = pawn_cap_east(pwn_rank_7, params.board.turn) & opp_bb;
        // Promotion by capturing west of pawn
        let mut m3 = pawn_cap_west(pwn_rank_7, params.board.turn) & opp_bb;

        if gen_type == &GenType::Evasions || gen_type == &GenType::EvadingCaptures {
            m1 &= target;
            m2 &= target;
            m3 &= target;
        }

        while m1 != 0 {
            let dest = BitBoard::pop_lsb(&mut m1);
            let src = dest - pawn_dir;
            make_promotions(params, src, dest, gen_type, false, move_list);
        }

        while m2 != 0 {
            let dest = BitBoard::pop_lsb(&mut m2);
            let src = dest - pawn_dir - 1;
            make_promotions(params, src, dest, gen_type, true, move_list);
        }

        while m3 != 0 {
            let dest = BitBoard::pop_lsb(&mut m3);
            let src = dest - pawn_dir + 1;
            make_promotions(params, src, dest, gen_type, true, move_list);
        }
    }
}

// Generate sliding and knight moves
fn gen_piece_moves(
    params: &MovegenParams,
    piece: PieceType,
    target: u64,
    checks: bool,
    move_list: &mut MoveList,
) {
    let opp = params.board.turn.opp();
    let mut piece_bb = params.board.player_piece_bb(params.board.turn, piece);

    while piece_bb != 0 {
        let sq = BitBoard::pop_lsb(&mut piece_bb);
        let mut bb = attacks(piece, sq, params.board.occ_bb(), params.board.turn) & target;

        if checks {
            // Moving a blocker also causes check
            if piece == PieceType::Queen
                || params.board.pos.king_blockers[opp.as_usize()] & BitBoard::from_sq(sq) == 0
            {
                bb &= params.board.pos.check_squares[piece.as_usize()];
            }
        }

        make_moves(params, sq, bb, params.board.player_bb(opp), move_list);
    }
}

fn generate_all_moves(gen_type: GenType, params: &MovegenParams, move_list: &mut MoveList) {
    let king_sq = params.board.cur_king_square();
    let checker_sq = BitBoard::bit_scan_forward(params.board.pos.checkers_bb);
    let checks = gen_type == GenType::QuietChecks;
    let mut target_bb = BitBoard::EMPTY;

    // Don' t generate piece moves in double check
    if (gen_type != GenType::Evasions && gen_type != GenType::EvadingCaptures)
        || !BitBoard::more_than_one(params.board.pos.checkers_bb)
    {
        target_bb = match gen_type {
            GenType::Evasions => between(king_sq, checker_sq) | BitBoard::from_sq(checker_sq),
            GenType::EvadingCaptures => {
                (between(king_sq, checker_sq) | BitBoard::from_sq(checker_sq))
                    & params.board.player_bb(params.board.turn.opp())
            }
            GenType::NonEvasions => !params.board.cur_player_bb(),
            GenType::Captures => params.board.player_bb(params.board.turn.opp()),
            GenType::Quiets | GenType::QuietChecks => !params.board.occ_bb(),
        };

        gen_pawn_moves(params, target_bb, &gen_type, move_list);
        gen_piece_moves(params, PieceType::Knight, target_bb, checks, move_list);
        gen_piece_moves(params, PieceType::Bishop, target_bb, checks, move_list);
        gen_piece_moves(params, PieceType::Rook, target_bb, checks, move_list);
        gen_piece_moves(params, PieceType::Queen, target_bb, checks, move_list);
    }

    if !checks || BitBoard::contains(params.board.blockers(params.board.turn.opp()), king_sq) {
        let mut bb = king_attacks(king_sq);
        if gen_type == GenType::Evasions {
            bb &= !params.board.cur_player_bb();
        } else {
            bb &= target_bb;
        }

        if checks {
            // Check by moving a blocked piece
            let opp_king_sq = params.board.king_square(params.board.turn.opp());
            bb &= !between(king_sq, opp_king_sq);
        }

        let opp_bb = params.board.player_bb(params.board.turn.opp());
        while bb != 0 {
            let dest = BitBoard::pop_lsb(&mut bb);
            if BitBoard::contains(opp_bb, dest) {
                let m = BitMove::from_flag(king_sq, dest, MoveFlag::CAPTURE);
                add_move(m, params, move_list);
            } else {
                let m = BitMove::from_squares(king_sq, dest);
                add_move(m, params, move_list);
            }
        }

        // Castling
        if (gen_type == GenType::Quiets || gen_type == GenType::NonEvasions)
            && !params.board.in_check()
            && params.board.can_castle(params.board.turn)
        {
            let occ = params.board.occ_bb();
            if params.board.can_castle_king(params.board.turn)
                && !BitBoard::contains(occ, king_sq + 1)
                && !BitBoard::contains(occ, king_sq + 2)
            {
                let m = BitMove::from_flag(king_sq, king_sq + 2, MoveFlag::CASTLE_KING);
                add_move(m, params, move_list);
            }
            if params.board.can_castle_queen(params.board.turn)
                && !BitBoard::contains(occ, king_sq - 1)
                && !BitBoard::contains(occ, king_sq - 2)
                && !BitBoard::contains(occ, king_sq - 3)
            {
                let m = BitMove::from_flag(king_sq, king_sq - 2, MoveFlag::CASTLE_QUEEN);
                add_move(m, params, move_list);
            }
        }
    }
}

pub fn generate_all(params: &MovegenParams, move_list: &mut MoveList) {
    if params.board.in_check() {
        generate_all_moves(GenType::Evasions, params, move_list);
    } else {
        generate_all_moves(GenType::NonEvasions, params, move_list);
    }
}

/// Wrapper around [`generate_all`]
pub fn generate_legal(params: &MovegenParams, move_list: &mut MoveList) {
    let mut pseudo = MoveList::new();
    generate_all(params, &mut pseudo);

    let mut i = 0;
    while i < pseudo.size() {
        let (m, score) = pseudo.get_all(i);
        if is_legal_move(params.board, m) {
            move_list.push(m, score);
        }

        i += 1;
    }
}

pub fn generate_quiet(params: &MovegenParams, move_list: &mut MoveList) {
    if params.board.in_check() {
        generate_all_moves(GenType::EvadingCaptures, params, move_list);
    } else {
        generate_all_moves(GenType::Captures, params, move_list);
        generate_all_moves(GenType::QuietChecks, params, move_list);
    }
}

pub const fn is_legal_move(board: &Board, m: u16) -> bool {
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
            let bishop_like_bb = board.player_piece_like_bb(board.turn.opp(), PieceType::Bishop);
            let rook_like_bb = board.player_piece_like_bb(board.turn.opp(), PieceType::Rook);

            bishop_attacks(king_sq, occ) & bishop_like_bb == 0
                && rook_attacks(king_sq, occ) & rook_like_bb == 0
        }
        _ => !BitBoard::contains(blockers, src) || BitBoard::triple_aligned(src, dest, king_sq),
    }
}

pub const fn smallest_attacker(board: &Board, sq: Square, side: Player) -> (PieceType, Square) {
    let pawns = pawn_attacks(sq, side) & board.player_piece_bb(side, PieceType::Pawn);
    if pawns != 0 {
        return (PieceType::Pawn, BitBoard::bit_scan_forward(pawns));
    }
    let knights = knight_attacks(sq) & board.player_piece_bb(side, PieceType::Knight);
    if knights != 0 {
        return (PieceType::Knight, BitBoard::bit_scan_forward(knights));
    }

    let occ = board.occ_bb();

    let bishop_moves = bishop_attacks(sq, occ);
    let bishops = bishop_moves & board.player_piece_bb(side, PieceType::Bishop);
    if bishops != 0 {
        return (PieceType::Bishop, BitBoard::bit_scan_forward(bishops));
    }

    let rook_moves = rook_attacks(sq, occ);
    let rooks = rook_moves & board.player_piece_bb(side, PieceType::Rook);
    if rooks != 0 {
        return (PieceType::Rook, BitBoard::bit_scan_forward(rooks));
    }

    let queens = (bishop_moves | rook_moves) & board.player_piece_bb(side, PieceType::Queen);
    if queens != 0 {
        return (PieceType::Queen, BitBoard::bit_scan_forward(queens));
    }

    let king = king_attacks(sq) & board.player_piece_bb(side, PieceType::King);
    if king != 0 {
        return (PieceType::King, BitBoard::bit_scan_forward(king));
    }

    (PieceType::None, 64)
}
