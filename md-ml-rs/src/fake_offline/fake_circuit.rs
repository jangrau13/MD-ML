// By Boshi Yuan (Rust rewrite)

use std::sync::{Arc, Mutex};

use crate::share::Spdz2kShare;
use crate::fake_offline::fake_gate::*;
use crate::fake_offline::*;

pub struct FakeCircuit<S: Spdz2kShare, const N: usize> {
    gates: Vec<FakeGateRef<S, N>>,
    endpoints: Vec<FakeGateRef<S, N>>,
}

impl<S: Spdz2kShare, const N: usize> FakeCircuit<S, N> {
    pub fn new() -> Self {
        FakeCircuit {
            gates: Vec::new(),
            endpoints: Vec::new(),
        }
    }

    pub fn run_offline(&self, party: &mut FakeParty<S, N>) {
        let t = std::time::Instant::now();
        for gate in &self.endpoints {
            run_fake_offline(gate, party);
        }
        eprintln!("[fake-offline] Total: {} ms", t.elapsed().as_millis());
    }

    pub fn add_endpoint(&mut self, gate: FakeGateRef<S, N>) {
        self.endpoints.push(gate);
    }

    pub fn input(
        &mut self,
        owner_id: usize,
        dim_row: usize,
        dim_col: usize,
    ) -> FakeGateRef<S, N> {
        let gate: FakeGateRef<S, N> =
            Arc::new(Mutex::new(FakeInputGate::new(dim_row, dim_col, owner_id)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn add(
        &mut self,
        input_x: FakeGateRef<S, N>,
        input_y: FakeGateRef<S, N>,
    ) -> FakeGateRef<S, N> {
        let gate: FakeGateRef<S, N> =
            Arc::new(Mutex::new(FakeAddGate::new(input_x, input_y)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn subtract(
        &mut self,
        input_x: FakeGateRef<S, N>,
        input_y: FakeGateRef<S, N>,
    ) -> FakeGateRef<S, N> {
        let gate: FakeGateRef<S, N> =
            Arc::new(Mutex::new(FakeSubtractGate::new(input_x, input_y)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn add_constant(
        &mut self,
        input_x: FakeGateRef<S, N>,
        constant: S::ClearType,
    ) -> FakeGateRef<S, N> {
        let gate: FakeGateRef<S, N> =
            Arc::new(Mutex::new(FakeAddConstantGate::new(input_x, constant)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn multiply(
        &mut self,
        input_x: FakeGateRef<S, N>,
        input_y: FakeGateRef<S, N>,
    ) -> FakeGateRef<S, N> {
        let gate: FakeGateRef<S, N> =
            Arc::new(Mutex::new(FakeMultiplyGate::new(input_x, input_y)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn output(&mut self, input_x: FakeGateRef<S, N>) -> FakeGateRef<S, N> {
        let gate: FakeGateRef<S, N> =
            Arc::new(Mutex::new(FakeOutputGate::new(input_x)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn multiply_trunc(
        &mut self,
        input_x: FakeGateRef<S, N>,
        input_y: FakeGateRef<S, N>,
    ) -> FakeGateRef<S, N> {
        let gate: FakeGateRef<S, N> =
            Arc::new(Mutex::new(FakeMultiplyTruncGate::new(input_x, input_y)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn element_multiply(
        &mut self,
        input_x: FakeGateRef<S, N>,
        input_y: FakeGateRef<S, N>,
    ) -> FakeGateRef<S, N> {
        let gate: FakeGateRef<S, N> =
            Arc::new(Mutex::new(FakeElemMultiplyGate::new(input_x, input_y)));
        self.gates.push(gate.clone());
        gate
    }

    pub fn gtz(&mut self, input_x: FakeGateRef<S, N>) -> FakeGateRef<S, N> {
        let gate: FakeGateRef<S, N> =
            Arc::new(Mutex::new(FakeGtzGate::new(input_x)));
        self.gates.push(gate.clone());
        gate
    }
}
