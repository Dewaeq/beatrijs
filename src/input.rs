use std::io;

use crate::{board::Board, search::Searcher, perft::perft, tests::{self, perft::test_perft}};

pub struct Game {
    board: Board,
    searcher: Searcher,
}

impl Game {
    fn new() -> Self {
        Game {
            board: Board::start_pos(),
            searcher: Searcher::new(Board::start_pos()),
        }
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

            let commands: Vec<&str> = buffer.split_whitespace().collect();
            let base_command = commands[0];

            if base_command == "d" {
                println!("{:?}", game.board);
            } else if base_command == "position" {
                game.parse_position(commands);
            } else if base_command == "search" {
                game.parse_search(commands);
            } else if base_command == "perft" {
                game.parse_perft(commands);
            } else if base_command == "test" {
                game.parse_test(commands);
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
        self.searcher = Searcher::new(self.board);
        let (time, score) = self.searcher.search(depth);
        let time = time as u64;

        println!(
            "info depth {depth} cp {score} nodes {} time {time} nps {}",
            self.searcher.num_nodes,
            (self.searcher.num_nodes as f64 / time as f64 * 1000f64) as u64
        );
    }

    fn parse_perft(&mut self, commands: Vec<&str>) {
        assert!(commands.len() == 3);
        assert!(commands[1] == "depth");

        let depth = commands[2].parse::<u8>().unwrap();
        perft(&mut self.board, depth, true);
    }

    fn parse_test(&mut self, commands: Vec<&str>) {
        assert!(commands.len() == 2);

        if commands[1] == "perft" {
            test_perft();
        }
    }
}
