use std::{process::exit, sync::atomic::Ordering, thread::JoinHandle, time::Instant};

use crate::search::MAX_SEARCH_DEPTH;
use crate::{bitmove::BitMove, board::Board, input::Game, search::SearchInfo};

/// Gui to engine
impl Game {
    pub fn uci(&mut self) {
        self.clear();
        self.id();
        self.uci_ok()
    }

    pub fn is_ready(&self) {
        self.ready_ok();
    }

    pub fn set_option(&mut self) {
        todo!()
    }

    pub fn uci_new_game(&mut self) {
        self.clear();
    }

    pub fn position(&mut self, commands: Vec<&str>) {
        let moves_idx = commands.iter().position(|&x| x == "moves");

        if commands.contains(&"fen") {
            let fen_str = match moves_idx {
                Some(idx) => commands[2..idx].join(" "),
                None => commands[2..].join(" "),
            };

            self.board = Board::from_fen(&fen_str);
        } else if commands.contains(&"startpos") {
            self.board = Board::start_pos();
        }

        match moves_idx {
            Some(idx) => self.make_moves(&commands[(idx + 1)..]),
            _ => (),
        }
    }

    pub fn go(&mut self, commands: Vec<&str>) {
        let mut info = SearchInfo::default();

        for mut i in 0..commands.len() {
            let command = commands[i];
            match command {
                "infinite" => info.depth = MAX_SEARCH_DEPTH,
                "depth" => {
                    info.depth = commands[i + 1].parse::<usize>().unwrap();
                    i += 1;
                }
                "wtime" => {
                    info.w_time = commands[i + 1].parse::<usize>().unwrap();
                    i += 1;
                }
                "btime" => {
                    info.b_time = commands[i + 1].parse::<usize>().unwrap();
                    i += 1;
                }
                "winc" => {
                    info.w_inc = commands[i + 1].parse::<usize>().unwrap();
                    i += 1;
                }
                "binc" => {
                    info.b_inc = commands[i + 1].parse::<usize>().unwrap();
                    i += 1;
                }
                _ => (),
            }
        }

        self.start_search(info);

        // TODO: improve time management
        if info.my_time(self.board.turn) > 0 {
            info.start();
            while info.has_time(self.board.turn) {}
            self.stop();
        }
    }

    pub fn stop(&mut self) {
        self.abort_search.store(true, Ordering::Relaxed);
        self.search_thread.take().map(JoinHandle::join);
    }

    pub fn quit(&mut self) {
        self.stop();
        exit(0);
    }
}

/// Engine to Gui
impl Game {
    pub fn id(&self) {
        println!("id name beatrijs author Dewaeq");
    }

    pub fn uci_ok(&self) {
        println!("uciok");
    }

    pub fn ready_ok(&self) {
        println!("readyok");
    }

    pub fn best_move(&self) {
        let best_move = self.table.best_move(self.board.key());
        println!("bestmove {}", BitMove::pretty_move(best_move.unwrap_or(0)));
    }
}
