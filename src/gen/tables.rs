use crate::{
    defs::{NUM_SQUARES, Score},
    utils::{b_max, coord_from_square},
};

/// Manhattan distance:
///
/// The minimal number of orthogonal king moves needed to go from square `a` to square `b`
pub const DISTANCE: [[Score; NUM_SQUARES]; NUM_SQUARES] = gen_distance();

const fn gen_distance() -> [[Score; NUM_SQUARES]; NUM_SQUARES] {
    let mut table = [[0; NUM_SQUARES]; NUM_SQUARES];

    let mut src = 0;
    while src < 64 {
        let (src_file, src_rank) = coord_from_square(src);

        let mut dest = 0;
        while dest < 64 {
            let (dest_file, dest_rank) = coord_from_square(dest);
            let dist = b_max((dest_rank - src_rank).abs(), (dest_file - src_file).abs());

            table[src as usize][dest as usize] = dist as Score;
            dest += 1;
        }

        src += 1;
    }

    table
}

#[rustfmt::skip]
/// Center Manhattan distance:
/// 
/// The minimal number of orthogonal king moves, on the otherwise empty board,
/// needed to reach one of the four center squares
pub const CENTER_DISTANCE: [Score; NUM_SQUARES] = [
  6, 5, 4, 3, 3, 4, 5, 6,
  5, 4, 3, 2, 2, 3, 4, 5,
  4, 3, 2, 1, 1, 2, 3, 4,
  3, 2, 1, 0, 0, 1, 2, 3,
  3, 2, 1, 0, 0, 1, 2, 3,
  4, 3, 2, 1, 1, 2, 3, 4,
  5, 4, 3, 2, 2, 3, 4, 5,
  6, 5, 4, 3, 3, 4, 5, 6
];
