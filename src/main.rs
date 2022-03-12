mod bitboard;
mod bitmove;
mod board;
mod defs;
mod gen;
mod history;
mod makemove;
mod movegen;
mod movelist;
mod perft;
mod position;
mod utils;

use crate::{perft::perft, movelist::MoveList, bitmove::BitMove};
use board::Board;

fn main() {
    let mut board = Board::start_pos();
    // let mut board = Board::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");
    // let mut board = Board::from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");
    // let mut board = Board::from_fen("8/2p5/1p1p4/K6r/1R3p1k/8/4P1P1/8 w - - 0 1");
    // let mut board = Board::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");

    println!("{:?}", board);
    // unsafe {
    //     let bresult = &mut *std::mem::MaybeUninit::<Board>::uninit().as_mut_ptr();
    //     *bresult = board;
    //     println!("{:?}", bresult);
    //     let moves = MoveList::legal(bresult);

    //     for m in moves {
    //         println!("{}", BitMove::pretty_move(m));
    //     }
    // }

    perft(&mut board, 6);
}
