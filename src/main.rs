#![allow(unused)]

mod bitboard;
mod bitmove;
mod board;
mod defs;
mod eval;
mod gen;
mod heuristics;
mod history;
mod input;
mod movegen;
mod movelist;
mod order;
mod params;
mod perft;
mod position;
mod psqt;
mod search;
mod search_info;
mod table;
mod tests;
mod uci;
mod utils;
mod zobrist;

use crate::input::Game;
pub(crate) use defs::e;

fn main() {
    Game::main_loop();
}
