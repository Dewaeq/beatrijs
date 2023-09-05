use crate::{
    bitboard::BitBoard,
    defs::{Dir, Square, DIRS},
    utils::{b_max, b_min, coord_from_square, is_in_board},
};

pub const RAY: [[u64; 64]; Dir::N_DIRS] = gen_rays();
pub const LINE: [[u64; 64]; 64] = gen_lines();
pub const SQ_TO_EDGE: [[Square; 64]; Dir::N_DIRS] = gen_sq_to_edge();

pub const ORTHOGONALS: [u64; 64] = gen_orthogonals();
pub const DIAGONALS: [u64; 64] = gen_diagonals();

pub const fn ray(dir_idx: usize, square: Square) -> u64 {
    if square == 64 {
        return 0;
    }
    RAY[dir_idx][square as usize]
}

pub const fn line(a: Square, b: Square) -> u64 {
    LINE[a as usize][b as usize]
}

const fn gen_rays() -> [[u64; 64]; Dir::N_DIRS] {
    let mut ray = [[0; 64]; Dir::N_DIRS];
    let mut dir_idx = 0;

    while dir_idx < 8 {
        let mut src = 0;

        while src < 64 {
            let dir = DIRS[dir_idx];
            let mut bb = BitBoard::EMPTY;
            let mut square = src;
            let mut i = 0;

            while i < SQ_TO_EDGE[dir_idx][src as usize] {
                square += dir;
                bb |= BitBoard::from_sq(square);
                i += 1;
            }

            ray[dir_idx][src as usize] = bb;
            src += 1;
        }

        dir_idx += 1;
    }

    ray
}

const fn get_line_bb(src: Square, offset: Square) -> u64 {
    let mut result = 0;

    let mut i = 1;
    loop {
        // Conversion to i16, wouldn't compile otherwise because of a possible overflow
        let dest = ((src as i16) + (i as i16) * (offset as i16)) as Square;
        let (x, y) = coord_from_square(src);
        let (t_x, t_y) = coord_from_square(dest);

        let x_dis = (x - t_x).abs();
        let y_dis = (y - t_y).abs();
        let move_dist = b_max(x_dis, y_dis);

        if move_dist == i && is_in_board(dest) {
            result |= BitBoard::from_sq(dest);
        } else {
            break;
        }
        i += 1;
    }

    i = 0;
    loop {
        // Conversion to i16, wouldn't compile otherwise because of a possible overflow
        let dest = ((src as i16) - (i as i16) * (offset as i16)) as Square;
        let (x, y) = coord_from_square(src);
        let (t_x, t_y) = coord_from_square(dest);

        let x_dis = (x - t_x).abs();
        let y_dis = (y - t_y).abs();
        let move_dist = b_max(x_dis, y_dis);

        if move_dist == i && is_in_board(dest) {
            result |= BitBoard::from_sq(dest);
        } else {
            break;
        }
        i += 1;
    }

    result
}

const fn gen_lines() -> [[u64; 64]; 64] {
    let mut lines = [[0; 64]; 64];
    let mut src = 0;

    while src < 64 {
        let (source_file, source_rank) = coord_from_square(src);
        let mut dest = 0;

        while dest < 64 {
            if src == dest {
                dest += 1;
            }

            let (dest_file, dest_rank) = coord_from_square(dest);

            // Bishop-like ray
            if (source_file - dest_file).abs() == (source_rank - dest_rank).abs() {
                let offset = if source_file > dest_file {
                    if source_rank > dest_rank {
                        DIRS[Dir::SOUTH_WEST]
                    } else {
                        DIRS[Dir::NORTH_WEST]
                    }
                } else if source_rank > dest_rank {
                    DIRS[Dir::SOUTH_EAST]
                } else {
                    DIRS[Dir::NORTH_EAST]
                };

                let bb = get_line_bb(src, offset);
                lines[src as usize][dest as usize] = bb;
            }
            // Rook-like ray
            else if (source_file == dest_file) || (source_rank == dest_rank) {
                let offset = if source_file > dest_file {
                    DIRS[Dir::WEST]
                } else if source_file < dest_file {
                    DIRS[Dir::EAST]
                } else if source_rank > dest_rank {
                    DIRS[Dir::SOUTH]
                } else {
                    DIRS[Dir::NORTH]
                };

                let bb = get_line_bb(src, offset);
                lines[src as usize][dest as usize] = bb;
            }

            dest += 1;
        }

        src += 1;
    }

    lines
}

const fn gen_sq_to_edge() -> [[Square; 64]; Dir::N_DIRS] {
    let mut sq_to_edge = [[0; 64]; Dir::N_DIRS];
    let mut src = 0;

    while src < 64 {
        let (x, y) = coord_from_square(src);
        let num_north = 7 - y;
        let num_south = y;
        let num_west = x;
        let num_east = 7 - x;
        let num_north_west = b_min(num_north, num_west);
        let num_south_east = b_min(num_south, num_east);
        let num_north_east = b_min(num_north, num_east);
        let num_south_west = b_min(num_south, num_west);

        sq_to_edge[Dir::NORTH][src as usize] = num_north;
        sq_to_edge[Dir::NORTH_EAST][src as usize] = num_north_east;
        sq_to_edge[Dir::EAST][src as usize] = num_east;
        sq_to_edge[Dir::SOUTH_EAST][src as usize] = num_south_east;
        sq_to_edge[Dir::SOUTH][src as usize] = num_south;
        sq_to_edge[Dir::SOUTH_WEST][src as usize] = num_south_west;
        sq_to_edge[Dir::WEST][src as usize] = num_west;
        sq_to_edge[Dir::NORTH_WEST][src as usize] = num_north_west;

        src += 1;
    }

    sq_to_edge
}

const fn gen_orthogonals() -> [u64; 64] {
    let mut table = [0; 64];

    let mut sq = 0;
    while sq < 64 {
        table[sq as usize] = BitBoard::file_bb(sq) | BitBoard::rank_bb(sq);
        sq += 1;
    }

    table
}

const fn gen_diagonals() -> [u64; 64] {
    let mut table = [0; 64];

    let mut sq = 0;
    while sq < 64 {
        table[sq as usize] |= BitBoard::from_sq(sq);
        table[sq as usize] |= ray(Dir::NORTH_EAST, sq);
        table[sq as usize] |= ray(Dir::SOUTH_EAST, sq);
        table[sq as usize] |= ray(Dir::SOUTH_WEST, sq);
        table[sq as usize] |= ray(Dir::NORTH_WEST, sq);

        sq += 1;
    }

    table
}
