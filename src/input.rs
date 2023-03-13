use std::{io, thread};
use std::sync::{Arc, Mutex};

use crate::{
    bitmove::BitMove,
    board::Board,
    movelist::MoveList,
    perft::perft,
    search::{evaluate, Searcher},
    tests::{self, perft::test_perft},
    utils::square_from_string,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Game {
    board: Board,
    abort_search: Arc<AtomicBool>,
}

impl Game {
    fn new() -> Self {
        Game {
            board: Board::start_pos(),
            abort_search: Arc::new(AtomicBool::new(false)),
        }
    }

    fn create_searcher(&self) -> Searcher {
        let abort = self.abort_search.clone();
        Searcher::new(self.board, abort)
    }

    pub fn main_loop() {
        let mut game = Game::new();
        let stdin = io::stdin();

        loop {
            let mut buffer = String::new();
            let input = stdin.read_line(&mut buffer);

            if !input.is_ok() || buffer.is_empty() || buffer.trim().is_empty() {
                continue;
            }

            game.stop_search();

            let commands: Vec<&str> = buffer.split_whitespace().collect();
            let base_command = commands[0];

            if base_command == "d" {
                println!("{:?}", game.board);
            } else if base_command == "position" {
                game.parse_position(commands);
            } else if base_command == "search" {
                game.parse_search(commands);
            } else if base_command == "go" {
                game.parse_go();
            } else if base_command == "stop" {
                game.stop_search();
            } else if base_command == "perft" {
                game.parse_perft(commands);
            } else if base_command == "test" {
                game.parse_test(commands);
            } else if base_command == "static" {
                game.parse_static(commands);
            } else if base_command == "move" {
                game.parse_move(commands);
            } else if base_command == "moves" {
                game.parse_moves();
            }
        }
    }

    fn parse_position(&mut self, commands: Vec<&str>) {
        if commands.contains(&"fen") {
            let fen = commands[2..].join(" ");
            self.board = Board::from_fen(fen.trim());
        } else if commands.contains(&"startpos") {
            self.board = Board::start_pos();
        } else {
            eprintln!("Invalid position command!");
        }
    }

    fn parse_search(&mut self, commands: Vec<&str>) {
        assert!(commands.len() == 3);
        assert!(commands[1] == "depth");

        let depth = commands[2].parse::<u8>().unwrap();
        let mut searcher = self.create_searcher();

        let handle = thread::spawn(move || {
            searcher.start();
            searcher.search(depth, i32::MIN + 1, i32::MAX - 1);
        });
    }

    fn parse_go(&mut self) {
        let mut searcher = self.create_searcher();

        let handle = std::thread::spawn(move || {
            searcher.iterate();
        });
    }

    fn stop_search(&mut self) {
        self.abort_search.store(true, Ordering::Relaxed);
    }

    fn parse_perft(&mut self, commands: Vec<&str>) {
        assert!(commands.len() == 3);
        assert!(commands[1] == "depth");

        let depth = commands[2].parse::<u8>().unwrap();
        perft(&mut self.board, depth, true);
    }

    fn parse_test(&self, commands: Vec<&str>) {
        assert!(commands.len() == 2);

        if commands[1] == "perft" {
            test_perft();
        }
    }

    fn parse_static(&self, commands: Vec<&str>) {
        let eval = evaluate(&self.board);
        println!("{} cp", eval);
    }

    fn parse_move(&mut self, commands: Vec<&str>) {
        assert!(commands.len() == 2);

        let src = square_from_string(&commands[1][0..2]);
        let dest = square_from_string(&commands[1][2..4]);

        let mut moves = MoveList::legal(&mut self.board);
        let mut m = moves.find(|&x| BitMove::src(x) == src && BitMove::dest(x) == dest);
        if let Some(m) = m {
            self.board.make_move(m);
        } else {
            eprintln!("failed to parse move {}", commands[1]);
        }
    }

    fn parse_moves(&mut self) {
        let mut moves = MoveList::legal(&mut self.board);
        print!("{}: ", moves.size());

        for m in moves {
            print!("{}, ", BitMove::pretty_move(m));
        }

        println!();
    }
}
