// By Boshi Yuan (Rust rewrite)

use md_ml::share::Spdz2kShare64;
use md_ml::fake_offline::{FakeParty, FakeCircuit};

const FAKE_OFFLINE_DIR: &str = "../fake-offline-data";

fn main() {
    type S = Spdz2kShare64;

    let mut party = FakeParty::<S, 2>::new("test", FAKE_OFFLINE_DIR);
    let mut circuit = FakeCircuit::<S, 2>::new();

    // Test for truncation correctness
    let a = circuit.input(0, 1, 1);
    let b = circuit.input(0, 1, 1);
    let c = circuit.multiply_trunc(a, b);
    let d = circuit.output(c);

    circuit.add_endpoint(d);
    circuit.run_offline(&mut party);
}
