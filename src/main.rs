#![allow(unused)]
#![feature(const_slice_index)]

use std::time::Instant;

use crate::bitmove::{BitMove, MoveFlag};
use crate::board::Board;
use crate::input::Game;
use crate::movelist::MoveList;
use crate::perft::perft;
use crate::search::Searcher;
use crate::tests::perft::test_perft;

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
mod tests;
mod utils;
mod zobrist;

fn main() {
    Game::main_loop();
}
