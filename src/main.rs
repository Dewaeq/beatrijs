#![allow(unused)]
#![feature(const_slice_index)]

use std::time::Instant;

use crate::bitmove::{BitMove, MoveFlag};
use crate::board::Board;
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
mod movegen;
mod movelist;
mod order;
mod perft;
mod position;
mod search;
mod utils;
mod zobrist;
mod tests;

fn main() {
    test_perft();
    return;

    let mut board = Board::start_pos();
    // let board = &mut Board::from_fen("5Q2/1k5p/6p1/5p2/2P2B2/1P5P/P4PP1/6K1 b - - 2 41");
    // let board = &mut Board::from_fen("3qr1k1/p5b1/2p1pp1p/3p3N/6Q1/r6P/5PP1/1R4K1 w - - 0 27");

    println!("{board:?}");

    // perft(&mut board, 6);

    let start = Instant::now();

    let mut searcher = Searcher::new(board);
    let score = searcher.search(9);
    let end = start.elapsed();

    println!("Total time (ms):   {}", end.as_secs_f64() * 1000f64);
    println!(
        "Nodes/s        :   {}",
        (searcher.num_nodes as f64 / end.as_secs_f64()) as u64
    );

    println!("{score}");
    println!("{:?}", searcher.num_nodes);
}
