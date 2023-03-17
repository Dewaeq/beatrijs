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
mod eval;
mod psqt;

use defs::PIECES;

use crate::{input::Game, gen::pesto::MG_TABLE, utils::{square_from_string, mirror, square_to_string}};

fn main() {
    Game::main_loop();
}
