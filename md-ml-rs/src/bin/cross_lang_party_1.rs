// Rust party 1 for cross-language tests with C++ party 0.
// Usage: cross_lang_party_1 <test_name>
// Tests: multiply_trunc, multiply, add, gtz, matmul

use std::env;

use md_ml::share::Spdz2kShare64;
use md_ml::protocols::{PartyWithFakeOffline, Circuit};
use md_ml::utils::fixed_point::fix2double;

const FAKE_OFFLINE_DIR: &str = "../fake-offline-data";
const PORT: usize = 7070;

type S = Spdz2kShare64;
type ClearType = u64;

fn test_multiply_trunc() {
    let mut party = PartyWithFakeOffline::<S>::new(1, 2, PORT, "xtest_mt_rs", FAKE_OFFLINE_DIR);
    let mut circuit = Circuit::<S>::new();

    let a = circuit.input(0, 1, 1);
    let b = circuit.input(0, 1, 1);
    let c = circuit.multiply_trunc(a.clone() as _, b.clone() as _);
    let d = circuit.output(c);
    circuit.add_endpoint(d.clone() as _);

    circuit.read_offline_from_file(&mut party);
    circuit.run_online(&mut party);

    let output = d.lock().unwrap().get_clear();
    println!("RESULT:{:.6}", fix2double::<ClearType>(output[0]));
}

fn test_multiply() {
    let mut party = PartyWithFakeOffline::<S>::new(1, 2, PORT, "xtest_mul_rs", FAKE_OFFLINE_DIR);
    let mut circuit = Circuit::<S>::new();

    let a = circuit.input(0, 1, 1);
    let b = circuit.input(0, 1, 1);
    let c = circuit.multiply(a.clone() as _, b.clone() as _);
    let d = circuit.output(c);
    circuit.add_endpoint(d.clone() as _);

    circuit.read_offline_from_file(&mut party);
    circuit.run_online(&mut party);

    let output = d.lock().unwrap().get_clear();
    println!("RESULT:{}", output[0]);
}

fn test_add() {
    let mut party = PartyWithFakeOffline::<S>::new(1, 2, PORT, "xtest_add_rs", FAKE_OFFLINE_DIR);
    let mut circuit = Circuit::<S>::new();

    let a = circuit.input(0, 1, 1);
    let b = circuit.input(0, 1, 1);
    let c = circuit.add(a.clone() as _, b.clone() as _);
    let d = circuit.output(c);
    circuit.add_endpoint(d.clone() as _);

    circuit.read_offline_from_file(&mut party);
    circuit.run_online(&mut party);

    let output = d.lock().unwrap().get_clear();
    println!("RESULT:{}", output[0]);
}

fn test_gtz() {
    let mut party = PartyWithFakeOffline::<S>::new(1, 2, PORT, "xtest_gtz_rs", FAKE_OFFLINE_DIR);
    let mut circuit = Circuit::<S>::new();

    let input_x = circuit.input(0, 10, 1);
    let g = circuit.gtz(input_x.clone() as _);
    let d = circuit.output(g);
    circuit.add_endpoint(d.clone() as _);

    circuit.read_offline_from_file(&mut party);
    circuit.run_online(&mut party);

    let output = d.lock().unwrap().get_clear();
    let strs: Vec<String> = output.iter().map(|v| v.to_string()).collect();
    println!("RESULT:{}", strs.join(","));
}

fn test_matmul() {
    let dim = 4;
    let mut party = PartyWithFakeOffline::<S>::new(1, 2, PORT, "xtest_mm_rs", FAKE_OFFLINE_DIR);
    let mut circuit = Circuit::<S>::new();

    let a = circuit.input(0, dim, dim);
    let b = circuit.input(0, dim, dim);
    let c = circuit.multiply_trunc(a.clone() as _, b.clone() as _);
    let d = circuit.output(c);
    circuit.add_endpoint(d.clone() as _);

    circuit.read_offline_from_file(&mut party);
    circuit.run_online(&mut party);

    let output = d.lock().unwrap().get_clear();
    let strs: Vec<String> = output.iter().map(|&v| {
        let d = fix2double::<ClearType>(v);
        // Match C++ cout default precision (6 significant digits)
        format!("{:.6}", d)
    }).collect();
    println!("RESULT:{}", strs.join(","));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <test_name>", args[0]);
        eprintln!("Tests: multiply_trunc, multiply, add, gtz, matmul");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "multiply_trunc" => test_multiply_trunc(),
        "multiply" => test_multiply(),
        "add" => test_add(),
        "gtz" => test_gtz(),
        "matmul" => test_matmul(),
        other => {
            eprintln!("Unknown test: {}", other);
            std::process::exit(1);
        }
    }
}
