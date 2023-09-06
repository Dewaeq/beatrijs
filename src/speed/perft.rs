use super::{board::Board, movegen::MoveGen};
use crate::bitmove::BitMove;
use std::time::Instant;

pub fn perft(board: &Board, depth: u8) -> u64 {
    let start = Instant::now();
    let nodes = inner_perft(true, board, depth);
    let end = start.elapsed();

    println!("\n=================================\n");
    println!("Total time (ms):   {}", end.as_secs_f64() * 1000f64);
    println!("Num moves      :   {}", MoveGen::simple(&board).size());
    println!("Num nodes      :   {nodes}");
    println!(
        "Nodes/s        :   {}",
        (nodes as f64 / end.as_secs_f64()) as u64
    );

    nodes
}

/// Only counts the number of leaf nodes
fn inner_perft(root: bool, board: &Board, depth: u8) -> u64 {
    let moves = MoveGen::simple(board);
    let mut count = 0;

    if depth == 0 {
        return 1;
    }

    for m in moves {
        let new_board = board.make_move(m);

        let add = if depth == 2 {
            MoveGen::simple(&new_board).size() as u64
        } else {
            inner_perft(false, &new_board, depth - 1)
        };

        count += add;

        if root {
            let pretty = BitMove::pretty_move(m);
            println!("{pretty}: {add}");
        }
    }

    count
}
