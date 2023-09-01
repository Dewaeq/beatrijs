use crate::table::TWrapper;
use std::sync::Arc;
use std::{process::exit, sync::atomic::Ordering, thread::JoinHandle};

use crate::search::MAX_SEARCH_DEPTH;
use crate::{board::Board, input::Game, search_info::SearchInfo};

/// Gui to engine
impl Game {
    pub fn uci(&mut self) {
        self.clear();
        println!("id name beatrijs author Dewaeq");
        println!("option name Hash type spin default 128 min 1 max 16384");
        println!("uciok");
    }

    pub fn is_ready(&self) {
        println!("readyok");
    }

    pub fn set_option(&mut self, commands: Vec<&str>) {
        let mut index = 1;
        while index < commands.len() {
            let option = commands[index];
            match option.to_lowercase().as_str() {
                "hash" => {
                    let size = commands[index + 2]
                        .parse()
                        .expect("Please provide a valid table size");
                    self.table = Arc::new(TWrapper::with_size(size));
                    return;
                }
                _ => index += 1,
            }
        }
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

        for i in 0..commands.len() {
            let command = commands[i];
            match command.to_lowercase().as_str() {
                "infinite" => {
                    info.depth = MAX_SEARCH_DEPTH;
                    info.time_set = false;
                }
                "depth" => {
                    info.depth = commands[i + 1].parse::<usize>().unwrap();
                }
                "movetime" => {
                    info.move_time = commands[i + 1].parse::<usize>().ok();
                    info.time_set = true;
                }
                "wtime" => {
                    info.w_time = commands[i + 1].parse::<usize>().ok();
                    info.time_set = true;
                }
                "btime" => {
                    info.b_time = commands[i + 1].parse::<usize>().ok();
                    info.time_set = true;
                }
                "winc" => {
                    info.w_inc = commands[i + 1].parse::<usize>().ok();
                    info.time_set = true;
                }
                "binc" => {
                    info.b_inc = commands[i + 1].parse::<usize>().ok();
                    info.time_set = true;
                }
                _ => (),
            }
        }

        self.start_search(info);
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

