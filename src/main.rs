#![allow(unused)]

use beatrijs::{board::Board, perft::{perft, perft_all}, search::{Searcher, evaluate}};

fn main() {
    let mut board = &mut Board::start_pos();
    // let board = &mut Board::from_fen("5Q2/1k5p/6p1/5p2/2P2B2/1P5P/P4PP1/6K1 b - - 2 41");
    // let board = &mut Board::from_fen("3qr1k1/p5b1/2p1pp1p/3p3N/6Q1/r6P/5PP1/1R4K1 w - - 0 27");
    
    println!("{:?}", board);
    perft(board, 6);

    // let mut searcher = Searcher::new(*board);
    // let score = searcher.search(7);
    // println!("{:?}", score);
    // println!("{:?}", searcher.num_nodes);
}
