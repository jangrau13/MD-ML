// By Boshi Yuan (Rust rewrite)

use std::time::Instant;

pub struct Timer {
    start: Option<Instant>,
    stop: Option<Instant>,
}

impl Timer {
    pub fn new() -> Self {
        Timer { start: None, stop: None }
    }

    pub fn start(&mut self) {
        self.start = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        self.stop = Some(Instant::now());
    }

    pub fn elapsed_ms(&self) -> u128 {
        match (self.start, self.stop) {
            (Some(s), Some(e)) => e.duration_since(s).as_millis(),
            _ => 0,
        }
    }

    pub fn print_elapsed(&self) {
        println!("Elapsed time: {} ms", self.elapsed_ms());
    }

    pub fn benchmark<F: FnOnce()>(&mut self, f: F) -> u128 {
        self.start();
        f();
        self.stop();
        self.elapsed_ms()
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}
