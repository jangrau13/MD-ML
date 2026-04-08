// By Boshi Yuan (Rust rewrite)

use std::sync::{Arc, Mutex};

use crate::share::Spdz2kShare;
use crate::protocols::gate::*;
use crate::protocols::*;
use crate::utils::timer::Timer;

pub struct Circuit<S: Spdz2kShare> {
    gates: Vec<GateRef<S>>,
    endpoints: Vec<GateRef<S>>,
    timer: Timer,
}

impl<S: Spdz2kShare> Circuit<S> {
    pub fn new() -> Self {
        Circuit {
            gates: Vec::new(),
            endpoints: Vec::new(),
            timer: Timer::new(),
        }
    }

    pub fn add_endpoint(&mut self, gate: GateRef<S>) {
        self.endpoints.push(gate);
    }

    pub fn read_offline_from_file(&self, party: &mut PartyWithFakeOffline<S>) {
        let t = std::time::Instant::now();
        for gate in &self.endpoints {
            read_offline_from_file(gate, party);
        }
        eprintln!("[offline] Total offline reading: {} ms", t.elapsed().as_millis());
    }

    pub fn run_online(&self, party: &mut PartyWithFakeOffline<S>) {
        for gate in &self.endpoints {
            run_online(gate, party);
        }
    }

    pub fn run_online_with_benchmark(&mut self, party: &mut PartyWithFakeOffline<S>) {
        self.timer.start();
        self.run_online(party);
        self.timer.stop();
    }

    pub fn print_stats(&self, party: &PartyWithFakeOffline<S>) {
        println!("Spent {} ms", self.timer.elapsed_ms());
        println!("Sent {} bytes", party.bytes_sent());
    }

    // Gate factory methods

    pub fn input(&mut self, owner_id: usize, dim_row: usize, dim_col: usize) -> Arc<Mutex<InputGate<S>>> {
        let gate = Arc::new(Mutex::new(InputGate::new(dim_row, dim_col, owner_id)));
        self.gates.push(gate.clone() as GateRef<S>);
        gate
    }

    pub fn add(&mut self, input_x: GateRef<S>, input_y: GateRef<S>) -> GateRef<S> {
        let gate: GateRef<S> = Arc::new(Mutex::new(AddGate::new(input_x, input_y)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn subtract(&mut self, input_x: GateRef<S>, input_y: GateRef<S>) -> GateRef<S> {
        let gate: GateRef<S> = Arc::new(Mutex::new(SubtractGate::new(input_x, input_y)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn add_constant(&mut self, input_x: GateRef<S>, constant: S::ClearType) -> GateRef<S> {
        let gate: GateRef<S> = Arc::new(Mutex::new(AddConstantGate::new(input_x, constant)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn multiply(&mut self, input_x: GateRef<S>, input_y: GateRef<S>) -> GateRef<S> {
        let gate: GateRef<S> = Arc::new(Mutex::new(MultiplyGate::new(input_x, input_y)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn output(&mut self, input_x: GateRef<S>) -> Arc<Mutex<OutputGate<S>>> {
        let gate = Arc::new(Mutex::new(OutputGate::new(input_x)));
        self.gates.push(gate.clone() as GateRef<S>);
        gate
    }

    pub fn multiply_trunc(&mut self, input_x: GateRef<S>, input_y: GateRef<S>) -> GateRef<S> {
        let gate: GateRef<S> = Arc::new(Mutex::new(MultiplyTruncGate::new(input_x, input_y)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn element_multiply(&mut self, input_x: GateRef<S>, input_y: GateRef<S>) -> GateRef<S> {
        let gate: GateRef<S> = Arc::new(Mutex::new(ElemMultiplyGate::new(input_x, input_y)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn gtz(&mut self, input_x: GateRef<S>) -> GateRef<S> {
        let gate: GateRef<S> = Arc::new(Mutex::new(GtzGate::new(input_x)));
        self.gates.push(gate.clone());
        gate
    }
}
