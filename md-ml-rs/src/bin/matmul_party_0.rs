// Matrix multiplication experiment - party 0 (Rust rewrite)

use md_ml::share::Spdz2kShare64;
use md_ml::protocols::{PartyWithFakeOffline, Circuit};
use rand::Rng;

const FAKE_OFFLINE_DIR: &str = "../fake-offline-data";
const JOB_NAME: &str = "MatMul";
const DIM: usize = 4096;

fn main() {
    type S = Spdz2kShare64;
    type ClearType = u64;

    let mut rng = rand::thread_rng();
    let mat_a: Vec<ClearType> = (0..DIM * DIM).map(|_| rng.gen::<ClearType>()).collect();
    let mat_b: Vec<ClearType> = (0..DIM * DIM).map(|_| rng.gen::<ClearType>()).collect();

    // Compute expected result locally (wrapping u64 arithmetic)
    let expected = {
        let mut out = vec![0u64; DIM * DIM];
        for i in 0..DIM {
            for k in 0..DIM {
                let a = mat_a[i * DIM + k];
                for j in 0..DIM {
                    out[i * DIM + j] = out[i * DIM + j].wrapping_add(a.wrapping_mul(mat_b[k * DIM + j]));
                }
            }
        }
        out
    };

    let mut party = PartyWithFakeOffline::<S>::new(0, 2, 6060, JOB_NAME, FAKE_OFFLINE_DIR);
    let mut circuit = Circuit::<S>::new();

    let a = circuit.input(0, DIM, DIM);
    let b = circuit.input(0, DIM, DIM);
    let c = circuit.multiply(a.clone(), b.clone());
    let d = circuit.output(c);
    circuit.add_endpoint(d.clone());

    a.lock().unwrap().set_input(&mat_a, 0);
    b.lock().unwrap().set_input(&mat_b, 0);

    circuit.read_offline_from_file(&mut party);
    circuit.run_online_with_benchmark(&mut party);
    circuit.print_stats(&party);

    // Correctness check
    let result = d.lock().unwrap().get_clear();
    let mut errors = 0;
    for (i, (&got, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        if got != exp {
            if errors < 10 {
                let row = i / DIM;
                let col = i % DIM;
                eprintln!("MISMATCH at ({}, {}): got {}, expected {}", row, col, got, exp);
            }
            errors += 1;
        }
    }
    if errors == 0 {
        println!("Correctness check PASSED: all {} elements match", result.len());
    } else {
        println!("Correctness check FAILED: {}/{} elements wrong", errors, result.len());
    }
}
