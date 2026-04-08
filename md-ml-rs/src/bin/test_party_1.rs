// By Boshi Yuan (Rust rewrite)

use md_ml::share::Spdz2kShare64;
use md_ml::protocols::{PartyWithFakeOffline, Circuit};

const FAKE_OFFLINE_DIR: &str = "../fake-offline-data";

fn main() {
    type S = Spdz2kShare64;

    let mut party = PartyWithFakeOffline::<S>::new(1, 2, 5050, "test", FAKE_OFFLINE_DIR);
    let mut circuit = Circuit::<S>::new();

    // Tests for truncation correctness
    let a = circuit.input(0, 1, 1);
    let b = circuit.input(0, 1, 1);
    let c = circuit.multiply_trunc(a, b);
    let d = circuit.output(c);
    circuit.add_endpoint(d);

    circuit.read_offline_from_file(&mut party);
    circuit.run_online_with_benchmark(&mut party);
    circuit.print_stats(&party);
}
