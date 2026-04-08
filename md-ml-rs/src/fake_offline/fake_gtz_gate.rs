// By Boshi Yuan (Rust rewrite)

use std::io::Write;

use crate::share::{ShareElement, Spdz2kShare};
use crate::fake_offline::fake_gate::*;
use crate::fake_offline::FakeParty;

pub struct FakeGtzGate<S: Spdz2kShare, const N: usize> {
    pub data: FakeGateData<S, N>,
}

impl<S: Spdz2kShare, const N: usize> FakeGtzGate<S, N> {
    pub fn new(input_x: FakeGateRef<S, N>) -> Self {
        let (dim_row, dim_col) = {
            let g = input_x.lock().unwrap();
            (g.dim_row(), g.dim_col())
        };
        let mut data = FakeGateData::new_with_inputs(input_x, None);
        data.dim_row = dim_row;
        data.dim_col = dim_col;
        FakeGtzGate { data }
    }

    fn generate_boolean_shares(x: S::ClearType) -> [S::ClearType; N] {
        let mut ret = [S::ClearType::zero(); N];
        let mut val = x;
        for i in 0..N - 1 {
            ret[i] = S::ClearType::random();
            val = val ^ ret[i];
        }
        ret[N - 1] = val;
        ret
    }
}

impl<S: Spdz2kShare, const N: usize> FakeGate<S, N> for FakeGtzGate<S, N> {
    impl_fake_gate_common!(FakeGtzGate, S, N);

    fn do_run_offline(&mut self, party: &mut FakeParty<S, N>) {
        let size = self.data.dim_row * self.data.dim_col;

        self.data.lambda_clear = vec![S::ClearType::zero(); size];
        S::ClearType::fill_random(&mut self.data.lambda_clear);

        let shares_and_macs = party.generate_all_parties_shares_vec(&self.data.lambda_clear);
        self.data.lambda_shr = shares_and_macs.value_shares;
        self.data.lambda_shr_mac = shares_and_macs.mac_shares;

        party.write_shares_to_all_parties(&self.data.lambda_shr);
        party.write_shares_to_all_parties(&self.data.lambda_shr_mac);

        // Boolean shares of lambda_x
        // Collect per-party shares, then write in bulk
        let mut bool_shares: [Vec<S::ClearType>; N] =
            std::array::from_fn(|_| vec![S::ClearType::zero(); size]);
        for idx in 0..size {
            let shares = Self::generate_boolean_shares(self.data.lambda_clear[idx]);
            for party_idx in 0..N {
                bool_shares[party_idx][idx] = shares[party_idx];
            }
        }
        for party_idx in 0..N {
            let bytes = S::ClearType::slice_as_bytes(&bool_shares[party_idx]);
            party.ith_party_file(party_idx).write_all(bytes)
                .expect("Failed to write boolean shares");
        }
    }
}
