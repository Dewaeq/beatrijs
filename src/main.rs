#![allow(unused)]
#![feature(const_slice_index)]
#![feature(sync_unsafe_cell)]

mod bitboard;
mod bitmove;
mod board;
mod defs;
mod gen;
mod history;
mod input;
mod movegen;
mod movelist;
mod order;
mod perft;
mod position;
mod search;
mod table;
mod tests;
mod utils;
mod zobrist;

use crate::input::Game;

fn main() {
    Game::main_loop();
}
