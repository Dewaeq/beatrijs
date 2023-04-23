use std::time::{Duration, Instant};

use crate::{defs::Player, search::MAX_SEARCH_DEPTH};

#[derive(Clone, Copy, Debug)]
pub struct SearchInfo {
    pub depth: usize,
    pub w_time: usize,
    pub b_time: usize,
    pub w_inc: usize,
    pub b_inc: usize,
    pub move_time: usize,
    pub time_set: bool,
    pub started: Instant,
    pub stop_time: Instant,
}

impl Default for SearchInfo {
    fn default() -> Self {
        Self {
            depth: MAX_SEARCH_DEPTH,
            w_time: 0,
            b_time: 0,
            w_inc: 0,
            b_inc: 0,
            move_time: 0,
            time_set: false,
            started: Instant::now(),
            stop_time: Instant::now(),
        }
    }
}

impl SearchInfo {
    pub fn with_depth(depth: usize) -> Self {
        let mut info = SearchInfo::default();
        info.depth = depth;
        info
    }

    pub fn my_time(&self, side: Player) -> usize {
        match side {
            Player::White => self.w_time,
            Player::Black => self.b_time,
        }
    }

    pub fn has_time(&self) -> bool {
        if !self.time_set {
            true
        } else {
            Instant::now() < self.stop_time
        }
    }

    pub fn start(&mut self, side: Player) {
        self.started = Instant::now();
        let search_time = Duration::from_millis((self.my_time(side) / 30) as u64);
        self.stop_time = Instant::now() + search_time;
    }
}
