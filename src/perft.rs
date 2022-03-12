use std::time::Instant;

use crate::{bitmove::BitMove, board::Board, movelist::MoveList};

#[derive(Debug)]
pub struct PerftResult {
    pub nodes: u64,
    pub captures: u64,
    pub en_passants: u64,
    pub castles: u64,
    pub promotions: u64,
    pub checks: u64,
    pub check_mates: u64,
}

pub fn perft_all(board: &mut Board, depth: u8) -> PerftResult {
    let mut perft = PerftResult {
        nodes: 0,
        captures: 0,
        en_passants: 0,
        castles: 0,
        promotions: 0,
        checks: 0,
        check_mates: 0,
    };

    inner_perft_all(board, depth, &mut perft);

    perft
}

pub fn perft(board: &mut Board, depth: u8) -> u64 {
    let start = Instant::now();
    let nodes = inner_perft(true, board, depth);
    let end = start.elapsed();

    println!("\n=================================\n");
    println!("Total time (s):   {}", end.as_secs_f64());
    println!("Num moves     :   {}", MoveList::legal(board).size());
    println!("Num nodes     :   {}", nodes);
    println!(
        "Nodes/s       :   {}",
        (nodes as f64 / end.as_secs_f64()) as u64
    );

    nodes
}

fn inner_perft_all(board: &mut Board, depth: u8, perft: &mut PerftResult) {
    let moves = MoveList::legal(board);

    if depth == 0 {
        perft.nodes += 1;
        if board.in_check() {
            if moves.is_empty() {
                perft.check_mates += 1;
            }
        }
    } else {
        for m in moves {
            if depth == 1 {
                if BitMove::is_cap(m) {
                    perft.captures += 1
                }
                if BitMove::is_ep(m) {
                    perft.en_passants += 1
                }
                if BitMove::is_castle(m) {
                    perft.castles += 1
                }
                if BitMove::is_prom(m) {
                    perft.promotions += 1
                }
            }

            let mut b = board.clone();
            b.make_move(m);
            inner_perft_all(&mut b, depth - 1, perft);
        }
    }
}

/// Only counts the number of leaf nodes
fn inner_perft(root: bool, board: &mut Board, depth: u8) -> u64 {
    let moves = MoveList::legal(board);
    let mut count = 0;

    if depth == 0 {
        return 1;
    }

    for m in moves {
        let b = &mut board.clone();
        b.make_move(m);

        let add = if depth == 2 {
            MoveList::legal(b).size() as u64
        } else {
            inner_perft(false, b, depth - 1)
        };

        count += add;

        if root {
            let pretty = BitMove::pretty_move(m);
            println!("{}: {}", pretty, add);
        }
    }

    count
}
