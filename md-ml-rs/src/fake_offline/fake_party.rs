// By Boshi Yuan (Rust rewrite)

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use crate::share::{ShareElement, Spdz2kShare, widen};

/// Stores the shares of a value and its MAC held by all N parties
pub struct AllPartiesShares<S: Spdz2kShare, const N: usize> {
    pub value_shares: [S::SemiShrType; N],
    pub mac_shares: [S::SemiShrType; N],
}

/// Vectorized version of AllPartiesShares
pub struct AllPartiesSharesVec<S: Spdz2kShare, const N: usize> {
    pub value_shares: [Vec<S::SemiShrType>; N],
    pub mac_shares: [Vec<S::SemiShrType>; N],
}

pub struct FakeParty<S: Spdz2kShare, const N: usize> {
    global_key: S::GlobalKeyType,
    global_key_as_semi: S::SemiShrType,
    key_shares: [S::KeyShrType; N],
    output_files: Vec<BufWriter<File>>,
}

impl<S: Spdz2kShare, const N: usize> FakeParty<S, N> {
    pub fn new(job_name: &str, fake_offline_dir: &str) -> Self {
        let dir = PathBuf::from(fake_offline_dir);
        if !dir.exists() {
            fs::create_dir_all(&dir).expect("Failed to create fake offline directory");
        }

        let sep = if job_name.is_empty() { "" } else { "-" };
        let mut output_files = Vec::with_capacity(N);
        for i in 0..N {
            let file_name = format!("{}{}party-{}.txt", job_name, sep, i);
            let path = dir.join(file_name);
            let file = File::create(&path).unwrap_or_else(|e| {
                panic!("Failed to create {:?}: {}", path, e)
            });
            // 8MB buffer to minimize syscalls
            output_files.push(BufWriter::with_capacity(8 * 1024 * 1024, file));
        }

        // Generate MAC key
        let mut global_key = S::GlobalKeyType::zero();
        let mut key_shares = [S::KeyShrType::zero(); N];
        for i in 0..N {
            key_shares[i] = S::KeyShrType::random();
            let widened = widen::<S::KeyShrType, S::GlobalKeyType>(key_shares[i]);
            global_key = global_key.wrapping_add(widened);
        }

        // Pre-compute widened global key
        let global_key_as_semi = widen::<S::GlobalKeyType, S::SemiShrType>(global_key);

        // Write MAC key shares to files (binary format)
        // Widen to GlobalKeyType to match the reader (which reads GlobalKeyType)
        for i in 0..N {
            let widened_key = widen::<S::KeyShrType, S::GlobalKeyType>(key_shares[i]);
            let bytes = widened_key.to_le_bytes_vec();
            output_files[i].write_all(&bytes).expect("Failed to write key");
        }

        FakeParty {
            global_key,
            global_key_as_semi,
            key_shares,
            output_files,
        }
    }

    #[inline]
    pub fn generate_all_parties_shares(&self, value: S::ClearType) -> AllPartiesShares<S, N> {
        let mask = S::KeyShrType::random();
        let masked_value: S::SemiShrType = {
            let mask_wide = widen::<S::KeyShrType, S::SemiShrType>(mask);
            let value_wide = widen::<S::ClearType, S::SemiShrType>(value);
            (mask_wide << S::K_BITS).wrapping_add(value_wide)
        };

        let mac = masked_value.wrapping_mul(self.global_key_as_semi);

        // Generate value shares
        let mut value_shares = [S::SemiShrType::zero(); N];
        let mut sum = S::SemiShrType::zero();
        for i in 0..N - 1 {
            value_shares[i] = S::SemiShrType::random();
            sum = sum.wrapping_add(value_shares[i]);
        }
        value_shares[N - 1] = masked_value.wrapping_sub(sum);

        // Generate MAC shares
        let mut mac_shares = [S::SemiShrType::zero(); N];
        let mut mac_sum = S::SemiShrType::zero();
        for i in 0..N - 1 {
            mac_shares[i] = S::SemiShrType::random();
            mac_sum = mac_sum.wrapping_add(mac_shares[i]);
        }
        mac_shares[N - 1] = mac.wrapping_sub(mac_sum);

        AllPartiesShares {
            value_shares,
            mac_shares,
        }
    }

    /// Vectorized share generation with bulk random number generation.
    ///
    /// This is the trusted dealer: it generates all N parties' shares at once
    /// (one file per party). The output is identical to calling
    /// `generate_all_parties_shares` per-element, but uses `fill_random` to
    /// generate all random bytes in one shot instead of per-element `random()`.
    pub fn generate_all_parties_shares_vec(
        &self,
        values: &[S::ClearType],
    ) -> AllPartiesSharesVec<S, N> {
        let size = values.len();

        // Pre-generate all random data in bulk (one RNG call per array)
        let mut masks = vec![S::KeyShrType::zero(); size];
        S::KeyShrType::fill_random(&mut masks);

        let mut value_shares: [Vec<S::SemiShrType>; N] =
            std::array::from_fn(|_| vec![S::SemiShrType::zero(); size]);
        let mut mac_shares: [Vec<S::SemiShrType>; N] =
            std::array::from_fn(|_| vec![S::SemiShrType::zero(); size]);

        // For parties 0..N-2, shares are uniformly random
        for i in 0..N - 1 {
            S::SemiShrType::fill_random(&mut value_shares[i]);
            S::SemiShrType::fill_random(&mut mac_shares[i]);
        }

        let global_key = self.global_key_as_semi;

        // Party N-1's share = (masked_value - sum_of_others) so that shares reconstruct correctly
        for idx in 0..size {
            let mask_wide = widen::<S::KeyShrType, S::SemiShrType>(masks[idx]);
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

    pub fn write_shares_to_all_parties(&mut self, shares: &[Vec<S::SemiShrType>; N]) {
        for party_idx in 0..N {
            let bytes = S::SemiShrType::slice_as_bytes(&shares[party_idx]);
            self.output_files[party_idx].write_all(bytes)
                .expect("Failed to write shares");
        }
    }

    pub fn write_shares_to_ith_party(&mut self, values: &[S::SemiShrType], party_id: usize) {
        let bytes = S::SemiShrType::slice_as_bytes(values);
        self.output_files[party_id].write_all(bytes)
            .expect("Failed to write shares");
    }

    pub fn write_clear_to_ith_party(&mut self, values: &[S::ClearType], party_id: usize) {
        let bytes = S::ClearType::slice_as_bytes(values);
        self.output_files[party_id].write_all(bytes)
            .expect("Failed to write clear values");
    }

    /// Write clear values to all parties, widened to SemiShrType for binary format
    /// compatibility (online reader always reads SemiShrType).
    pub fn write_clear_to_all_parties(&mut self, values: &[S::ClearType]) {
        let widened: Vec<S::SemiShrType> = values.iter()
            .map(|&v| widen::<S::ClearType, S::SemiShrType>(v))
            .collect();
        let bytes = S::SemiShrType::slice_as_bytes(&widened);
        for party_idx in 0..N {
            self.output_files[party_idx].write_all(bytes)
                .expect("Failed to write clear values");
        }
    }

    pub fn ith_party_file(&mut self, i: usize) -> &mut BufWriter<File> {
        &mut self.output_files[i]
    }
}
