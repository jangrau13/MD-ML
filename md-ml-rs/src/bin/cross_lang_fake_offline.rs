// Dual-format fake offline generator for cross-language testing.
// Generates preprocessing data in BOTH text (for C++) and binary (for Rust) formats,
// using the same random values, so that a C++ party and Rust party can communicate
// over TCP and produce the same result.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use md_ml::share::{ShareElement, Spdz2kShare, Spdz2kShare64, widen};
use md_ml::utils::fixed_point::truncate_clear_vec;
use md_ml::utils::linear_algebra::*;

const FAKE_OFFLINE_DIR: &str = "../fake-offline-data";

/// A dual-format fake party that writes both text (C++) and binary (Rust) files.
struct DualFakeParty<S: Spdz2kShare, const N: usize> {
    global_key_as_semi: S::SemiShrType,
    // Text writers (for C++)
    text_files: Vec<BufWriter<File>>,
    // Binary writers (for Rust)
    bin_files: Vec<BufWriter<File>>,
}

impl<S: Spdz2kShare, const N: usize> DualFakeParty<S, N> {
    fn new(cpp_job: &str, rs_job: &str) -> Self {
        let dir = PathBuf::from(FAKE_OFFLINE_DIR);
        if !dir.exists() {
            fs::create_dir_all(&dir).expect("Failed to create fake offline directory");
        }

        // Generate MAC key
        let mut global_key = S::GlobalKeyType::zero();
        let mut key_shares = vec![S::KeyShrType::zero(); N];
        for i in 0..N {
            key_shares[i] = S::KeyShrType::random();
            let widened = widen::<S::KeyShrType, S::GlobalKeyType>(key_shares[i]);
            global_key = global_key.wrapping_add(widened);
        }
        let global_key_as_semi = widen::<S::GlobalKeyType, S::SemiShrType>(global_key);

        // Open files and write MAC key shares
        let mut text_files = Vec::with_capacity(N);
        let mut bin_files = Vec::with_capacity(N);

        for i in 0..N {
            // Text file for C++
            let text_name = format!("{}-party-{}.txt", cpp_job, i);
            let text_path = dir.join(&text_name);
            let text_file = File::create(&text_path)
                .unwrap_or_else(|e| panic!("Failed to create {:?}: {}", text_path, e));
            let mut tw = BufWriter::with_capacity(8 * 1024 * 1024, text_file);
            // C++ FakeParty writes key_shares[i] (KeyShrType = u64) as decimal text
            write!(tw, "{}\n", key_shares[i]).unwrap();
            text_files.push(tw);

            // Binary file for Rust
            let bin_name = format!("{}-party-{}.txt", rs_job, i);
            let bin_path = dir.join(&bin_name);
            let bin_file = File::create(&bin_path)
                .unwrap_or_else(|e| panic!("Failed to create {:?}: {}", bin_path, e));
            let mut bw = BufWriter::with_capacity(8 * 1024 * 1024, bin_file);
            // Rust FakeParty writes widen(key_shares[i]) as GlobalKeyType binary LE
            let widened_key = widen::<S::KeyShrType, S::GlobalKeyType>(key_shares[i]);
            bw.write_all(&widened_key.to_le_bytes_vec()).unwrap();
            bin_files.push(bw);
        }

        DualFakeParty {
            global_key_as_semi,
            text_files,
            bin_files,
        }
    }

    fn generate_shares_vec(
        &self,
        values: &[S::ClearType],
    ) -> AllPartiesSharesVec<S, N> {
        let size = values.len();
        let mut value_shares: Vec<Vec<S::SemiShrType>> =
            (0..N).map(|_| vec![S::SemiShrType::zero(); size]).collect();
        let mut mac_shares: Vec<Vec<S::SemiShrType>> =
            (0..N).map(|_| vec![S::SemiShrType::zero(); size]).collect();

        // Random shares for parties 0..N-2
        for i in 0..N - 1 {
            S::SemiShrType::fill_random(&mut value_shares[i]);
            S::SemiShrType::fill_random(&mut mac_shares[i]);
        }

        let global_key = self.global_key_as_semi;
        for idx in 0..size {
            let mask = S::KeyShrType::random();
            let mask_wide = widen::<S::KeyShrType, S::SemiShrType>(mask);
            let value_wide = widen::<S::ClearType, S::SemiShrType>(values[idx]);
            let masked_value = (mask_wide << S::K_BITS).wrapping_add(value_wide);
            let mac = masked_value.wrapping_mul(global_key);

            let mut v_sum = S::SemiShrType::zero();
            let mut m_sum = S::SemiShrType::zero();
            for i in 0..N - 1 {
                v_sum = v_sum.wrapping_add(value_shares[i][idx]);
                m_sum = m_sum.wrapping_add(mac_shares[i][idx]);
            }
            value_shares[N - 1][idx] = masked_value.wrapping_sub(v_sum);
            mac_shares[N - 1][idx] = mac.wrapping_sub(m_sum);
        }

        AllPartiesSharesVec {
            value_shares,
            mac_shares,
        }
    }

    /// Write SemiShrType shares to all parties in both formats
    fn write_shares_all(&mut self, shares: &[Vec<S::SemiShrType>]) {
        assert_eq!(shares.len(), N);
        for party_idx in 0..N {
            // Text: one decimal number per line
            for &v in &shares[party_idx] {
                write!(self.text_files[party_idx], "{}\n", v).unwrap();
            }
            // Binary: raw LE bytes
            let bytes = S::SemiShrType::slice_as_bytes(&shares[party_idx]);
            self.bin_files[party_idx].write_all(bytes).unwrap();
        }
    }

    /// Write ClearType values to the owner party in both formats
    /// C++ writes ClearType as decimal; Rust writes widened to SemiShrType as binary
    fn write_clear_to_owner(&mut self, values: &[S::ClearType], owner_id: usize) {
        // Text: ClearType as decimal
        for &v in values {
            write!(self.text_files[owner_id], "{}\n", v).unwrap();
        }
        // Binary: Rust FakeInputGate widens to SemiShrType
        let widened: Vec<S::SemiShrType> = values.iter()
            .map(|&v| widen::<S::ClearType, S::SemiShrType>(v))
            .collect();
        let bytes = S::SemiShrType::slice_as_bytes(&widened);
        self.bin_files[owner_id].write_all(bytes).unwrap();
    }

    /// Write ClearType values to all parties (used by FakeMultiplyGate for deltas)
    /// C++ writes ClearType as decimal; Rust writes widened to SemiShrType as binary
    fn write_clear_to_all(&mut self, values: &[S::ClearType]) {
        let widened: Vec<S::SemiShrType> = values.iter()
            .map(|&v| widen::<S::ClearType, S::SemiShrType>(v))
            .collect();
        let bytes = S::SemiShrType::slice_as_bytes(&widened);
        for party_idx in 0..N {
            // Text
            for &v in values {
                write!(self.text_files[party_idx], "{}\n", v).unwrap();
            }
            // Binary
            self.bin_files[party_idx].write_all(bytes).unwrap();
        }
    }

    /// Write ClearType boolean shares for GtzGate
    /// C++ writes ClearType as decimal per party; Rust writes ClearType as binary per party
    fn write_bool_shares_all(&mut self, shares: &[Vec<S::ClearType>]) {
        assert_eq!(shares.len(), N);
        for party_idx in 0..N {
            // Text
            for &v in &shares[party_idx] {
                write!(self.text_files[party_idx], "{}\n", v).unwrap();
            }
            // Binary
            let bytes = S::ClearType::slice_as_bytes(&shares[party_idx]);
            self.bin_files[party_idx].write_all(bytes).unwrap();
        }
    }

    fn flush(&mut self) {
        for f in &mut self.text_files {
            f.flush().unwrap();
        }
        for f in &mut self.bin_files {
            f.flush().unwrap();
        }
    }
}

struct AllPartiesSharesVec<S: Spdz2kShare, const N: usize> {
    value_shares: Vec<Vec<S::SemiShrType>>,
    mac_shares: Vec<Vec<S::SemiShrType>>,
}

// ---- Circuit generation helpers ----

struct LambdaData<S: Spdz2kShare, const N: usize> {
    clear: Vec<S::ClearType>,
    shr: Vec<Vec<S::SemiShrType>>,
    shr_mac: Vec<Vec<S::SemiShrType>>,
}

fn gen_input<S: Spdz2kShare, const N: usize>(
    party: &mut DualFakeParty<S, N>,
    size: usize,
    owner_id: usize,
) -> LambdaData<S, N> {
    let mut lambda_clear = vec![S::ClearType::zero(); size];
    S::ClearType::fill_random(&mut lambda_clear);

    let shares = party.generate_shares_vec(&lambda_clear);

    // Write: lambda_clear to owner, then shares, then macs
    party.write_clear_to_owner(&lambda_clear, owner_id);
    party.write_shares_all(&shares.value_shares);
    party.write_shares_all(&shares.mac_shares);

    LambdaData {
        clear: lambda_clear,
        shr: shares.value_shares,
        shr_mac: shares.mac_shares,
    }
}

fn gen_multiply<S: Spdz2kShare, const N: usize>(
    party: &mut DualFakeParty<S, N>,
    dim_row: usize,
    dim_mid: usize,
    dim_col: usize,
    input_x_lambda: &[S::ClearType],
    input_y_lambda: &[S::ClearType],
) -> LambdaData<S, N> {
    let size_lhs = dim_row * dim_mid;
    let size_rhs = dim_mid * dim_col;
    let size_output = dim_row * dim_col;

    // lambda_z = rand()
    let mut lambda_clear = vec![S::ClearType::zero(); size_output];
    S::ClearType::fill_random(&mut lambda_clear);
    let lambda_sm = party.generate_shares_vec(&lambda_clear);

    // Beaver triple: a, b, c = a*b
    let mut a_clear = vec![S::ClearType::zero(); size_lhs];
    let mut b_clear = vec![S::ClearType::zero(); size_rhs];
    S::ClearType::fill_random(&mut a_clear);
    S::ClearType::fill_random(&mut b_clear);
    let c_clear = matrix_multiply(&a_clear, &b_clear, dim_row, dim_mid, dim_col);

    let a_sm = party.generate_shares_vec(&a_clear);
    let b_sm = party.generate_shares_vec(&b_clear);
    let c_sm = party.generate_shares_vec(&c_clear);

    // delta_x = a - lambda_x, delta_y = b - lambda_y
    let delta_x = matrix_subtract(&a_clear, input_x_lambda);
    let delta_y = matrix_subtract(&b_clear, input_y_lambda);

    // Write order must match FakeMultiplyGate exactly:
    // a_shr, a_mac, b_shr, b_mac, c_shr, c_mac, lambda_shr, lambda_mac, delta_x, delta_y
    party.write_shares_all(&a_sm.value_shares);
    party.write_shares_all(&a_sm.mac_shares);
    party.write_shares_all(&b_sm.value_shares);
    party.write_shares_all(&b_sm.mac_shares);
    party.write_shares_all(&c_sm.value_shares);
    party.write_shares_all(&c_sm.mac_shares);
    party.write_shares_all(&lambda_sm.value_shares);
    party.write_shares_all(&lambda_sm.mac_shares);
    party.write_clear_to_all(&delta_x);
    party.write_clear_to_all(&delta_y);

    LambdaData {
        clear: lambda_clear,
        shr: lambda_sm.value_shares,
        shr_mac: lambda_sm.mac_shares,
    }
}

fn gen_multiply_trunc<S: Spdz2kShare, const N: usize>(
    party: &mut DualFakeParty<S, N>,
    dim_row: usize,
    dim_mid: usize,
    dim_col: usize,
    input_x_lambda: &[S::ClearType],
    input_y_lambda: &[S::ClearType],
) -> LambdaData<S, N> {
    // First do the base multiply generation
    let mut mul_data = gen_multiply::<S, N>(
        party, dim_row, dim_mid, dim_col, input_x_lambda, input_y_lambda,
    );

    // Compute truncated lambda
    let lambda_prime_clear = truncate_clear_vec(&mul_data.clear);
    let lambda_prime_sm = party.generate_shares_vec(&lambda_prime_clear);

    // Write lambda_prime shares
    party.write_shares_all(&lambda_prime_sm.value_shares);
    party.write_shares_all(&lambda_prime_sm.mac_shares);

    // Swap: truncated lambda becomes the "real" lambda
    mul_data.clear = lambda_prime_clear;
    mul_data.shr = lambda_prime_sm.value_shares;
    mul_data.shr_mac = lambda_prime_sm.mac_shares;

    mul_data
}

fn gen_add<S: Spdz2kShare, const N: usize>(
    party: &mut DualFakeParty<S, N>,
    input_x_lambda: &LambdaData<S, N>,
    input_y_lambda: &LambdaData<S, N>,
) -> LambdaData<S, N> {
    // Add gate: lambda_z = lambda_x + lambda_y
    let clear = matrix_add(&input_x_lambda.clear, &input_y_lambda.clear);
    let n = input_x_lambda.shr.len();
    let shr: Vec<Vec<S::SemiShrType>> = (0..n)
        .map(|i| matrix_add(&input_x_lambda.shr[i], &input_y_lambda.shr[i]))
        .collect();
    let shr_mac: Vec<Vec<S::SemiShrType>> = (0..n)
        .map(|i| matrix_add(&input_x_lambda.shr_mac[i], &input_y_lambda.shr_mac[i]))
        .collect();

    // FakeAddGate writes lambda_shr and lambda_shr_mac to file
    party.write_shares_all(&shr);
    party.write_shares_all(&shr_mac);

    LambdaData { clear, shr, shr_mac }
}

fn gen_output<S: Spdz2kShare, const N: usize>(
    _party: &mut DualFakeParty<S, N>,
    _lambda: &LambdaData<S, N>,
) {
    // Output gate writes nothing in the offline phase
}

fn gen_gtz<S: Spdz2kShare, const N: usize>(
    party: &mut DualFakeParty<S, N>,
    size: usize,
    input_lambda: &LambdaData<S, N>,
) -> LambdaData<S, N> {
    let mut lambda_clear = vec![S::ClearType::zero(); size];
    S::ClearType::fill_random(&mut lambda_clear);

    let shares = party.generate_shares_vec(&lambda_clear);

    // Write lambda shares and macs
    party.write_shares_all(&shares.value_shares);
    party.write_shares_all(&shares.mac_shares);

    // Boolean shares of input lambda (NOT this gate's lambda)
    let mut bool_shares: Vec<Vec<S::ClearType>> =
        (0..N).map(|_| vec![S::ClearType::zero(); size]).collect();
    for idx in 0..size {
        let val = input_lambda.clear[idx];
        let mut remaining = val;
        for i in 0..N - 1 {
            let r = S::ClearType::random();
            bool_shares[i][idx] = r;
            remaining = remaining ^ r;
        }
        bool_shares[N - 1][idx] = remaining;
    }
    party.write_bool_shares_all(&bool_shares);

    LambdaData {
        clear: lambda_clear,
        shr: shares.value_shares,
        shr_mac: shares.mac_shares,
    }
}

// ---- Test circuit generators ----

fn generate_test_multiply_trunc<S: Spdz2kShare>(cpp_job: &str, rs_job: &str) {
    let mut party = DualFakeParty::<S, 2>::new(cpp_job, rs_job);

    let a = gen_input::<S, 2>(&mut party, 1, 0);
    let b = gen_input::<S, 2>(&mut party, 1, 0);
    let _c = gen_multiply_trunc::<S, 2>(&mut party, 1, 1, 1, &a.clear, &b.clear);
    // output gate writes nothing offline

    party.flush();
    eprintln!("Generated multiply_trunc test");
}

fn generate_test_multiply<S: Spdz2kShare>(cpp_job: &str, rs_job: &str) {
    let mut party = DualFakeParty::<S, 2>::new(cpp_job, rs_job);

    let a = gen_input::<S, 2>(&mut party, 1, 0);
    let b = gen_input::<S, 2>(&mut party, 1, 0);
    let _c = gen_multiply::<S, 2>(&mut party, 1, 1, 1, &a.clear, &b.clear);

    party.flush();
    eprintln!("Generated multiply test");
}

fn generate_test_add<S: Spdz2kShare>(cpp_job: &str, rs_job: &str) {
    let mut party = DualFakeParty::<S, 2>::new(cpp_job, rs_job);

    let a = gen_input::<S, 2>(&mut party, 1, 0);
    let b = gen_input::<S, 2>(&mut party, 1, 0);
    let _c = gen_add::<S, 2>(&mut party, &a, &b);
    // output gate writes nothing offline

    party.flush();
    eprintln!("Generated add test");
}

fn generate_test_gtz<S: Spdz2kShare>(cpp_job: &str, rs_job: &str) {
    let mut party = DualFakeParty::<S, 2>::new(cpp_job, rs_job);

    let input = gen_input::<S, 2>(&mut party, 10, 0);
    let _gtz = gen_gtz::<S, 2>(&mut party, 10, &input);

    party.flush();
    eprintln!("Generated gtz test");
}

fn generate_test_matmul<S: Spdz2kShare>(cpp_job: &str, rs_job: &str) {
    let dim = 4;
    let mut party = DualFakeParty::<S, 2>::new(cpp_job, rs_job);

    let a = gen_input::<S, 2>(&mut party, dim * dim, 0);
    let b = gen_input::<S, 2>(&mut party, dim * dim, 0);
    let _c = gen_multiply_trunc::<S, 2>(&mut party, dim, dim, dim, &a.clear, &b.clear);

    party.flush();
    eprintln!("Generated matmul {}x{} test", dim, dim);
}

fn main() {
    eprintln!("Generating cross-language test preprocessing data...");

    generate_test_multiply_trunc::<Spdz2kShare64>("xtest_mt_cpp", "xtest_mt_rs");
    generate_test_multiply::<Spdz2kShare64>("xtest_mul_cpp", "xtest_mul_rs");
    generate_test_add::<Spdz2kShare64>("xtest_add_cpp", "xtest_add_rs");
    generate_test_gtz::<Spdz2kShare64>("xtest_gtz_cpp", "xtest_gtz_rs");
    generate_test_matmul::<Spdz2kShare64>("xtest_mm_cpp", "xtest_mm_rs");

    eprintln!("Done! Files written to {}/", FAKE_OFFLINE_DIR);
}
