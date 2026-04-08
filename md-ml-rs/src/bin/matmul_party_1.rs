// Matrix multiplication experiment - party 1 (Rust rewrite)

use md_ml::share::Spdz2kShare64;
use md_ml::protocols::{PartyWithFakeOffline, Circuit};

const FAKE_OFFLINE_DIR: &str = "../fake-offline-data";
const JOB_NAME: &str = "MatMul";
const DIM: usize = 4096;

fn main() {
    type S = Spdz2kShare64;

    let mut party = PartyWithFakeOffline::<S>::new(1, 2, 6060, JOB_NAME, FAKE_OFFLINE_DIR);
    let mut circuit = Circuit::<S>::new();

    let a = circuit.input(0, DIM, DIM);
    let b = circuit.input(0, DIM, DIM);
    let c = circuit.multiply(a, b);
    let d = circuit.output(c);
    circuit.add_endpoint(d);

    circuit.read_offline_from_file(&mut party);
    circuit.run_online_with_benchmark(&mut party);
    circuit.print_stats(&party);
}
