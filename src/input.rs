use std::sync::Arc;
use std::thread::JoinHandle;
use std::{io, thread};

use crate::eval::evaluate;
use crate::table::TWrapper;
use crate::{
    bitmove::BitMove, board::Board, movelist::MoveList, perft::perft, search::Searcher,
    tests::perft::test_perft, utils::square_from_string,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Game {
    board: Board,
    abort_search: Arc<AtomicBool>,
    search_thread: Option<JoinHandle<()>>,
    table: Arc<TWrapper>,
}

impl Game {
    fn new() -> Self {
        Game {
            board: Board::start_pos(),
            abort_search: Arc::new(AtomicBool::new(false)),
            search_thread: None,
            table: Arc::new(TWrapper::new()),
        }
    }

    fn create_searcher(&mut self) -> Searcher {
        let abort = self.abort_search.clone();
        let table = self.table.clone();
        Searcher::new(self.board, abort, table)
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
                game.parse_go(255);
            } else if base_command == "stop" {
                game.stop_search();
            } else if base_command == "perft" {
                game.parse_perft(commands);
            } else if base_command == "test" {
                game.parse_test(commands);
            } else if base_command == "static" {
                game.parse_static(commands);
            } else if base_command == "take" {
                game.board.unmake_last_move();
                println!("{:?}", game.board);
            } else if base_command == "move" {
                game.parse_move(commands);
            } else if base_command == "moves" {
                game.parse_moves();
            } else if base_command == "pv" {
                let pv = game.table.extract_pv(&mut game.board);

                print!("pv ");
                for m in pv {
                    print!("{} ", BitMove::pretty_move(m));
                }

                println!();
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

        self.table.clear();
    }

    fn parse_search(&mut self, commands: Vec<&str>) {
        assert!(commands.len() == 3);
        assert!(commands[1] == "depth");

        let depth = commands[2].parse::<u8>().unwrap();
        self.parse_go(depth);
    }

    fn parse_go(&mut self, max_depth: u8) {
        let mut searcher = self.create_searcher();
        let handle = thread::spawn(move || {
            searcher.start();
            searcher.iterate(max_depth);
        });

        self.search_thread = Some(handle);
    }

    fn stop_search(&mut self) {
        self.abort_search.store(true, Ordering::Release);
        self.search_thread.take().map(JoinHandle::join);
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
        assert!(commands.len() >= 2);

        let num_moves = commands.len();
        for i in 1..num_moves {
            let src = square_from_string(&commands[i][0..2]);
            let dest = square_from_string(&commands[i][2..4]);

            let mut moves = MoveList::legal(&mut self.board);
            let m = moves.find(|&x| BitMove::src(x) == src && BitMove::dest(x) == dest);
            if let Some(m) = m {
                self.board.make_move(m);
            } else {
                eprintln!("failed to parse move {}", commands[i]);
                break;
            }
        }
    }

    fn parse_moves(&mut self) {
        let moves = MoveList::legal(&mut self.board);
        print!("{}: ", moves.size());

        for m in moves {
            print!("{}, ", BitMove::pretty_move(m));
        }

        println!();
    }
}
