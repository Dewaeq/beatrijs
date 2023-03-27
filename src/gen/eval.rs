use crate::defs::{NUM_PIECES};

/// index by attacker and victim
pub const MVV_LVA: [[i32; NUM_PIECES]; NUM_PIECES] = gen_mvvlva();
const VICTIM_VALUE: [i32; NUM_PIECES] = [100, 200, 300, 400, 500, 600];

pub const fn gen_mvvlva() -> [[i32; NUM_PIECES]; NUM_PIECES] {
    let mut table = [[0; NUM_PIECES]; NUM_PIECES];

    let mut i = 0;
    while i < NUM_PIECES {
        let mut j = 0;
        while j < NUM_PIECES {
            table[i][j] = VICTIM_VALUE[j] + 6 - VICTIM_VALUE[i] / 100;

            j += 1;
        }

        i += 1;
    }

    table
}
