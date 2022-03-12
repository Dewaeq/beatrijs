use crate::{
    bitboard::BitBoard,
    defs::{Player, DIRS},
    utils::{b_max, coord_from_square, is_in_board},
};

pub const KING_ATK: [u64; 64] = gen_king();
pub const KNIGHT_ATK: [u64; 64] = gen_knight();
pub const PAWN_ATK: [[u64; 64]; 2] = gen_pawn();

const fn gen_king() -> [u64; 64] {
    let mut king_atk: [u64; 64] = [0; 64];
    let mut src = 0;

    while src < 64 {
        let (x, y) = coord_from_square(src);
        let mut dir_idx = 0;

        while dir_idx < DIRS.len() {
            let king_dir = DIRS[dir_idx];
            let t_sq = src + king_dir;
            let (t_x, t_y) = coord_from_square(t_sq);

            let x_dis = (x - t_x).abs();
            let y_dis = (y - t_y).abs();
            let move_dist = b_max(x_dis, y_dis);

            if move_dist == 1 && is_in_board(t_sq) {
                king_atk[src as usize] |= BitBoard::from_sq(t_sq);
            }

            dir_idx += 1;
        }

        src += 1;
    }

    king_atk
}

const fn gen_knight() -> [u64; 64] {
    let all_knight_dir: [i8; 8] = [17, 10, -6, -15, -17, -10, 6, 15];
    let mut knight_atk: [u64; 64] = [0; 64];
    let mut src = 0;

    while src < 64 {
        let (x, y) = coord_from_square(src);
        let mut dir_idx = 0;

        while dir_idx < 8 {
            let knight_dir = all_knight_dir[dir_idx];
            let t_sq = src + knight_dir;
            let (t_x, t_y) = coord_from_square(t_sq);

            let x_dis = (x - t_x).abs();
            let y_dis = (y - t_y).abs();
            let move_dist = b_max(x_dis, y_dis);

            if move_dist == 2 && is_in_board(t_sq) {
                knight_atk[src as usize] |= BitBoard::from_sq(t_sq);
            }

            dir_idx += 1;
        }

        src += 1;
    }

    knight_atk
}

const fn gen_pawn() -> [[u64; 64]; 2] {
    let mut pawn_atk: [[u64; 64]; 2] = [[0; 64]; 2];
    let mut src = 0;

    while src < 64 {
        let (x, y) = coord_from_square(src);
        let mut white_bb = BitBoard::EMPTY;
        let mut black_bb = BitBoard::EMPTY;

        // Take west of pawn
        if x > 0 {
            if y < 7 {
                white_bb |= BitBoard::from_sq(src + 7);
            }
            if y > 0 {
                black_bb |= BitBoard::from_sq(src - 9);
            }
        }
        // Take east of pawn
        if x < 7 {
            if y < 7 {
                white_bb |= BitBoard::from_sq(src + 9);
            }
            if y > 0 {
                black_bb |= BitBoard::from_sq(src - 7);
            }
        }

        pawn_atk[Player::White.as_usize()][src as usize] = white_bb;
        pawn_atk[Player::Black.as_usize()][src as usize] = black_bb;

        src += 1;
    }

    pawn_atk
}
