use std::sync::Arc;
use std::thread::JoinHandle;
use std::{io, thread};

use crate::defs::PieceType;
use crate::eval::evaluate;
use crate::search::SearchInfo;
use crate::table::TWrapper;
use crate::utils::print_pv;
use crate::{
    bitmove::BitMove, board::Board, movelist::MoveList, perft::perft, search::Searcher,
    tests::perft::test_perft, utils::square_from_string,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Game {
    pub board: Board,
    pub abort_search: Arc<AtomicBool>,
    pub search_thread: Option<JoinHandle<()>>,
    pub table: Arc<TWrapper>,
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

    pub fn clear(&mut self) {
        self.table.clear();
        self.stop();
        // self.board = Board::start_pos();
    }

    fn create_searcher(&mut self, info: SearchInfo) -> Searcher {
        let abort = self.abort_search.clone();
        let table = self.table.clone();
        Searcher::new(self.board, abort, table, info)
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
            game.parse_commands(commands);
        }
    }

    fn parse_commands(&mut self, commands: Vec<&str>) {
        let base_command = commands[0];

        // UCI commands
        if base_command == "uci" {
            self.uci();
        } else if base_command == "isready" {
            self.is_ready();
        } else if base_command == "ucinewgame" {
            self.uci_new_game();
        } else if base_command == "position" {
            self.position(commands);
        } else if base_command == "go" {
            self.go(commands);
        } else if base_command == "stop" {
            self.stop();
        } else if base_command == "quit" {
            self.quit();
        }
        // Custom commands
        else if base_command == "d" {
            println!("{:?}", self.board);
        } else if base_command == "perft" {
            self.parse_perft(commands);
        } else if base_command == "test" {
            self.parse_test(commands);
        } else if base_command == "static" {
            self.parse_static(commands);
        } else if base_command == "take" {
            self.board.unmake_last_move();
            println!("{:?}", self.board);
        } else if base_command == "move" {
            self.parse_move(commands);
        } else if base_command == "moves" {
            self.print_moves();
        }
    }

    pub fn start_search(&mut self, info: SearchInfo) {
        let mut searcher = self.create_searcher(info);
        let handle = thread::spawn(move || {
            searcher.iterate();
        });

        self.search_thread = Some(handle);
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

        self.make_moves(&commands[1..]);

        println!("{:?}", self.board);
    }

    fn print_moves(&mut self) {
        let moves = MoveList::legal(&mut self.board);
        print!("{}: ", moves.size());

        for m in moves {
            print!("{}, ", BitMove::pretty_move(m));
        }

        println!();
    }

    fn str_to_move(&mut self, move_str: &str) -> Option<u16> {
        assert!(move_str.len() == 4 || move_str.len() == 5);

        let src = square_from_string(&move_str[0..2]);
        let dest = square_from_string(&move_str[2..4]);
        let prom_type = match move_str.get(4..5) {
            Some("n") => PieceType::Knight,
            Some("b") => PieceType::Bishop,
            Some("r") => PieceType::Rook,
            Some("q") => PieceType::Queen,
            _ => PieceType::None,
        };

        let mut moves = MoveList::legal(&mut self.board);
        moves.find(|&x| {
            BitMove::src(x) == src
                && BitMove::dest(x) == dest
                && BitMove::prom_type(BitMove::flag(x)) == prom_type
        })
    }

    pub fn make_moves(&mut self, moves: &[&str]) {
        for move_str in moves {
            let bitmove = self.str_to_move(move_str);
            if let Some(m) = bitmove {
                self.board.make_move(m);
            } else {
                eprintln!("failed to parse move {}", move_str);
                return;
            }
        }
    }
}
