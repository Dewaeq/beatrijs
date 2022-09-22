mod bitboard;
mod bitmove;
mod board;
mod defs;
mod gen;
mod history;
mod makemove;
mod movegen;
mod movelist;
mod perft;
mod position;
mod utils;
mod zobrist;

use crate::perft::perft;
use board::Board;

fn main() {
    let mut board = Board::start_pos();

    println!("{:?}", board);

    perft(&mut board, 6);
}
