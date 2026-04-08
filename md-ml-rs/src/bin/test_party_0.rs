// By Boshi Yuan (Rust rewrite)

use md_ml::share::Spdz2kShare64;
use md_ml::protocols::{PartyWithFakeOffline, Circuit};
use md_ml::utils::fixed_point::{double2fix, fix2double};

const FAKE_OFFLINE_DIR: &str = "../fake-offline-data";

fn main() {
    type S = Spdz2kShare64;
    type ClearType = u64;

    let mut party = PartyWithFakeOffline::<S>::new(0, 2, 5050, "test", FAKE_OFFLINE_DIR);
    let mut circuit = Circuit::<S>::new();

    // Tests for truncation correctness
    let a = circuit.input(0, 1, 1);
    let b = circuit.input(0, 1, 1);
    let c = circuit.multiply_trunc(a.clone(), b.clone());
    let d = circuit.output(c);
    circuit.add_endpoint(d.clone());

    let val: ClearType = double2fix(1.5);
    a.lock().unwrap().set_input(&[val], 0);
    b.lock().unwrap().set_input(&[val], 0);

    circuit.read_offline_from_file(&mut party);
    circuit.run_online_with_benchmark(&mut party);
    circuit.print_stats(&party);

    let output = d.lock().unwrap().get_clear();
    println!("output: {}", fix2double::<ClearType>(output[0]));
}
