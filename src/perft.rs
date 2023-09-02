use crate::{
    bitmove::BitMove, board::Board, movegen::MovegenParams, movelist::MoveList,
};
use std::time::Instant;

#[derive(Debug)]
pub struct PerftResult {
    pub time: f64,
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
        time: 0f64,
        nodes: 0,
        captures: 0,
        en_passants: 0,
        castles: 0,
        promotions: 0,
        checks: 0,
        check_mates: 0,
    };

    let start = Instant::now();
    inner_perft_all(board, depth, &mut perft);
    let end = start.elapsed();

    perft.time = end.as_secs_f64() * 1000f64;

    perft
}

pub fn perft(board: &mut Board, depth: u8, print_info: bool) -> u64 {
    let start = Instant::now();
    let nodes = inner_perft(print_info, board, depth);
    let end = start.elapsed();

    if print_info {
        println!("\n=================================\n");
        println!("Total time (ms):   {}", end.as_secs_f64() * 1000f64);
        println!(
            "Num moves      :   {}",
            MoveList::legal(MovegenParams::simple(board)).size()
        );
        println!("Num nodes      :   {nodes}");
        println!(
            "Nodes/s        :   {}",
            (nodes as f64 / end.as_secs_f64()) as u64
        );
    }

    nodes
}

fn inner_perft_all(
    board: &mut Board,
    depth: u8,
    perft: &mut PerftResult,
) {
    let moves = MoveList::legal(MovegenParams::simple(board));

    if depth == 0 {
        perft.nodes += 1;
        if board.in_check() {
            perft.checks += 1;
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

            board.make_move(m);
            inner_perft_all(board, depth - 1, perft);
            board.unmake_move(m);
        }
    }
}

/// Only counts the number of leaf nodes
fn inner_perft(root: bool, board: &mut Board, depth: u8) -> u64 {
    let moves = MoveList::legal(MovegenParams::simple(board));
    let mut count = 0;

    if depth == 0 {
        return 1;
    }

    for m in moves {
        board.make_move(m);

        let add = if depth == 2 {
            MoveList::legal(MovegenParams::simple(board)).size() as u64
        } else {
            inner_perft(false, board, depth - 1)
        };

        board.unmake_move(m);

        count += add;

        if root {
            let pretty = BitMove::pretty_move(m);
            println!("{pretty}: {add}");
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use crate::{board::Board, perft::perft_all};

    fn perft_all_test(
        fen: &str,
        depth: u8,
        nodes: u64,
        captures: u64,
        en_passants: u64,
        castles: u64,
        promotions: u64,
        checks: u64,
        check_mates: u64,
    ) {
        println!("Testing {} at depth {}", fen, depth);

        let mut board = Board::from_fen(fen);
        let result = perft_all(&mut board, depth);

        assert_eq!(result.nodes, nodes);
        assert_eq!(result.captures, captures);
        assert_eq!(result.en_passants, en_passants);
        assert_eq!(result.castles, castles);
        assert_eq!(result.promotions, promotions);
        assert_eq!(result.checks, checks);
        assert_eq!(result.check_mates, check_mates);
    }

    #[test]
    fn perft_all_position_1() {
        perft_all_test(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            4,
            4085603,
            757163,
            1929,
            128013,
            15172,
            25523,
            43,
        )
    }
}
