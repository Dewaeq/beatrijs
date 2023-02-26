#![allow(unused)]

use std::time::Instant;

use crate::bitmove::{BitMove, MoveFlag};
use crate::board::Board;
use crate::movelist::MoveList;
use crate::perft::perft;
use crate::search::Searcher;

mod gen;
mod bitboard;
mod bitmove;
mod board;
mod defs;
mod history;
mod makemove;
mod movegen;
mod movelist;
mod perft;
mod position;
mod search;
mod utils;
mod zobrist;
mod order;

fn main() {
    let board = &mut Board::start_pos();
    // let board = &mut Board::from_fen("5Q2/1k5p/6p1/5p2/2P2B2/1P5P/P4PP1/6K1 w - - 2 41");
    // let board = &mut Board::from_fen("3qr1k1/p5b1/2p1pp1p/3p3N/6Q1/r6P/5PP1/1R4K1 w - - 0 27");
    
    /* let m1 = BitMove::from_squares(9, 17);
    let m2 = BitMove::from_squares(48, 40);
    let m3 = BitMove::from_squares(12, 20);
    let m4 = BitMove::from_squares(40, 32);
    let m5 = BitMove::from_squares(3, 30);
    let m6 = BitMove::from_squares(32, 24);
    let m7 = BitMove::from_squares(5, 33);
    let m8 = BitMove::from_flag(53, 37, MoveFlag::DOUBLE_PAWN_PUSH);
    let m9 = BitMove::from_flag(33, 51, MoveFlag::CAPTURE);

    board.make_move(m1);
    board.make_move(m2);
    board.make_move(m3);
    board.make_move(m4);
    board.make_move(m5);
    board.make_move(m6);
    board.make_move(m7);
    board.make_move(m8);
    board.make_move(m9);

    println!("{:?}", board);

    println!("legal quiets:");
    let quiets = MoveList::quiet(board);
    println!("{}", quiets.size());

    for m in quiets {
        println!("{}", BitMove::pretty_move(m));
    }

    println!("legal all:");
    let all = MoveList::legal(board);
    println!("{}", all.size());

    for m in all {
        println!("{}", BitMove::pretty_move(m));
    } */


    // perft(board, 6);

    let start = Instant::now();

    let mut searcher = Searcher::new(board.clone());
    let score = searcher.search(7);
    let end = start.elapsed();

    println!("Total time (ms):   {}", end.as_secs_f64() * 1000f64);
    println!(
        "Nodes/s        :   {}",
        (searcher.num_nodes as f64 / end.as_secs_f64()) as u64
    );

    println!("{:?}", score);
    println!("{:?}", searcher.num_nodes);
}
