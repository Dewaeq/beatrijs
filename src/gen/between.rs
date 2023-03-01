use crate::{
    bitboard::BitBoard,
    defs::{Dir, Square, DIRS},
    utils::coord_from_square,
};

pub const BETWEEN: [[u64; 64]; 64] = gen_between();

pub const fn between(source: Square, dest: Square) -> u64 {
    BETWEEN[source as usize][dest as usize]
}

const fn get_between_bb(source: Square, dest: Square, offset: Square) -> u64 {
    let mut result = 0;
    let mut cur_square = source + offset;

    while cur_square != dest {
        result |= BitBoard::from_sq(cur_square);
        cur_square += offset;
    }

    result
}

const fn gen_between() -> [[u64; 64]; 64] {
    let mut between = [[0; 64]; 64];
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

                let bb = get_between_bb(src, dest, offset);
                between[src as usize][dest as usize] = bb;
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

                let bb = get_between_bb(src, dest, offset);
                between[src as usize][dest as usize] = bb;
            }

            dest += 1;
        }

        src += 1;
    }

    between
}
