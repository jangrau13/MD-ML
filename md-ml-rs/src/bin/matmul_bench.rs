// Matrix multiplication benchmark (Rust rewrite)

use nalgebra::DMatrix;
use rand::Rng;
use md_ml::utils::timer::Timer;

fn bench_suite_u64(timer: &mut Timer) {
    let mut rng = rand::thread_rng();

    println!("\n=== uint64_t (used by SPDZ2k) ===");
    println!("{:<14}{:>10}{:>14}", "Size", "Time (ms)", "GFLOPS");
    println!("{}", "-".repeat(38));

    for &n in &[512, 1024, 2048, 4096] {
        let a: Vec<u64> = (0..n * n).map(|_| rng.gen()).collect();
        let b: Vec<u64> = (0..n * n).map(|_| rng.gen()).collect();

        let ma = DMatrix::from_row_slice(n, n, &a);
        let mb = DMatrix::from_row_slice(n, n, &b);

        let ms = timer.benchmark(|| {
            let _mc = &ma * &mb;
        });

        let flops = 2.0 * (n as f64).powi(3);
        let gflops = flops / (ms as f64 / 1000.0) / 1e9;

        println!("{:<14}{:>10}{:>12.2}", format!("{}x{}", n, n), ms, gflops);
    }
}

fn bench_suite_f64(timer: &mut Timer) {
    let mut rng = rand::thread_rng();

    println!("\n=== double ===");
    println!("{:<14}{:>10}{:>14}", "Size", "Time (ms)", "GFLOPS");
    println!("{}", "-".repeat(38));

    for &n in &[512, 1024, 2048, 4096] {
        let a: Vec<f64> = (0..n * n).map(|_| rng.gen_range(0.0..1.0)).collect();
        let b: Vec<f64> = (0..n * n).map(|_| rng.gen_range(0.0..1.0)).collect();

        let ma = DMatrix::from_row_slice(n, n, &a);
        let mb = DMatrix::from_row_slice(n, n, &b);

        let ms = timer.benchmark(|| {
            let _mc = &ma * &mb;
        });

        let flops = 2.0 * (n as f64).powi(3);
        let gflops = flops / (ms as f64 / 1000.0) / 1e9;

        println!("{:<14}{:>10}{:>12.2}", format!("{}x{}", n, n), ms, gflops);
    }
}

fn bench_suite_f32(timer: &mut Timer) {
    let mut rng = rand::thread_rng();

    println!("\n=== float ===");
    println!("{:<14}{:>10}{:>14}", "Size", "Time (ms)", "GFLOPS");
    println!("{}", "-".repeat(38));

    for &n in &[512, 1024, 2048, 4096] {
        let a: Vec<f32> = (0..n * n).map(|_| rng.gen_range(0.0..1.0f32)).collect();
        let b: Vec<f32> = (0..n * n).map(|_| rng.gen_range(0.0..1.0f32)).collect();

        let ma = DMatrix::from_row_slice(n, n, &a);
        let mb = DMatrix::from_row_slice(n, n, &b);

        let ms = timer.benchmark(|| {
            let _mc = &ma * &mb;
        });

        let flops = 2.0 * (n as f64).powi(3);
        let gflops = flops / (ms as f64 / 1000.0) / 1e9;

        println!("{:<14}{:>10}{:>12.2}", format!("{}x{}", n, n), ms, gflops);
    }
}

fn main() {
    let mut timer = Timer::new();

    bench_suite_u64(&mut timer);
    bench_suite_f64(&mut timer);
    bench_suite_f32(&mut timer);
}
