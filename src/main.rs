#![allow(unused)]
#![feature(const_slice_index)]
#![feature(sync_unsafe_cell)]

mod bitboard;
mod bitmove;
mod board;
mod defs;
mod eval;
mod gen;
mod history;
mod input;
mod movegen;
mod movelist;
mod order;
mod perft;
mod position;
mod psqt;
mod search;
mod table;
mod tests;
mod utils;
mod zobrist;
mod uci;

use crate::input::Game;

fn main() {
    Game::main_loop();
}
