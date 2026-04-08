// By Boshi Yuan (Rust rewrite)

use md_ml::share::Spdz2kShare64;
use md_ml::protocols::{PartyWithFakeOffline, Circuit};

const FAKE_OFFLINE_DIR: &str = "../fake-offline-data";
const JOB_NAME: &str = "DotProduct";
const DIM: usize = 65536;

fn main() {
    type S = Spdz2kShare64;
    type ClearType = u64;

    let vec: Vec<ClearType> = vec![1; DIM];

    let mut party = PartyWithFakeOffline::<S>::new(0, 2, 5050, JOB_NAME, FAKE_OFFLINE_DIR);
    let mut circuit = Circuit::<S>::new();

    let a = circuit.input(0, 1, DIM);
    let b = circuit.input(0, DIM, 1);
    let c = circuit.multiply(a.clone(), b.clone());
    let d = circuit.output(c);
    circuit.add_endpoint(d);

    a.lock().unwrap().set_input(&vec, 0);
    b.lock().unwrap().set_input(&vec, 0);

    circuit.read_offline_from_file(&mut party);
    circuit.run_online_with_benchmark(&mut party);
    circuit.print_stats(&party);
}
