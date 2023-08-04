use std::time::{Duration, Instant};

use crate::{defs::Player, search::MAX_SEARCH_DEPTH};

#[derive(Clone, Copy, Debug)]
pub struct SearchInfo {
    pub depth: usize,
    pub w_time: Option<usize>,
    pub b_time: Option<usize>,
    pub w_inc: Option<usize>,
    pub b_inc: Option<usize>,
    pub move_time: Option<usize>,
    pub time_set: bool,
    pub started: Instant,
    pub stop_time: Instant,
}

impl Default for SearchInfo {
    fn default() -> Self {
        Self {
            depth: MAX_SEARCH_DEPTH,
            w_time: None,
            b_time: None,
            w_inc: None,
            b_inc: None,
            move_time: None,
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

    pub fn my_time(&self, side: Player) -> Option<usize> {
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

        if self.time_set {
            let search_time = if let Some(move_time) = self.move_time {
                Duration::from_millis(move_time as u64)
            } else {
                let my_time = self.my_time(side).unwrap();
                Duration::from_millis((my_time / 30) as u64)
            };
            self.stop_time = Instant::now() + search_time;
        }
    }
}
