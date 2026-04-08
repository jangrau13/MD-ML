// Matrix multiplication experiment - fake offline phase (Rust rewrite)

use md_ml::share::Spdz2kShare64;
use md_ml::fake_offline::{FakeParty, FakeCircuit};

const FAKE_OFFLINE_DIR: &str = "../fake-offline-data";
const JOB_NAME: &str = "MatMul";
const DIM: usize = 4096;

fn main() {
    type S = Spdz2kShare64;

    let mut party = FakeParty::<S, 2>::new(JOB_NAME, FAKE_OFFLINE_DIR);
    let mut circuit = FakeCircuit::<S, 2>::new();

    let a = circuit.input(0, DIM, DIM);
    let b = circuit.input(0, DIM, DIM);
    let c = circuit.multiply(a, b);
    let d = circuit.output(c);

    circuit.add_endpoint(d);
    circuit.run_offline(&mut party);
}
