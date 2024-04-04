#![allow(unused)]
#![feature(const_slice_index)]
#![feature(sync_unsafe_cell)]
#![feature(const_fn_floating_point_arithmetic)]

mod bench;
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

use std::env;

use crate::input::Game;

fn main() {
    let mut args = std::env::args();
    if args.nth(1) == Some("bench".to_string()) {
        bench::run();
    } else {
        Game::main_loop();
    }
}
