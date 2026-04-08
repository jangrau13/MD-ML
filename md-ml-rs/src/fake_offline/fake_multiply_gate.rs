// By Boshi Yuan (Rust rewrite)

use crate::share::{ShareElement, Spdz2kShare};
use crate::fake_offline::fake_gate::*;
use crate::fake_offline::FakeParty;
use crate::utils::linear_algebra::*;
use std::time::Instant;

pub struct FakeMultiplyGate<S: Spdz2kShare, const N: usize> {
    pub data: FakeGateData<S, N>,
    pub dim_mid: usize,
}

impl<S: Spdz2kShare, const N: usize> FakeMultiplyGate<S, N> {
    pub fn new(input_x: FakeGateRef<S, N>, input_y: FakeGateRef<S, N>) -> Self {
        let (dim_row, dim_mid, dim_col) = {
            let gx = input_x.lock().unwrap();
            let gy = input_y.lock().unwrap();
            assert!(
                gx.dim_col() == gy.dim_row(),
                "The inputs of multiplication gate should have compatible dimensions"
            );
            (gx.dim_row(), gx.dim_col(), gy.dim_col())
        };
        let mut data = FakeGateData::new_with_inputs(input_x, Some(input_y));
        data.dim_row = dim_row;
        data.dim_col = dim_col;
        FakeMultiplyGate { data, dim_mid }
    }

    pub fn do_run_offline_base(&mut self, party: &mut FakeParty<S, N>) {
        let size_lhs = self.data.dim_row * self.dim_mid;
        let size_rhs = self.dim_mid * self.data.dim_col;
        let size_output = self.data.dim_row * self.data.dim_col;

        // lambda_z = rand()
        let t = Instant::now();
        self.data.lambda_clear = vec![S::ClearType::zero(); size_output];
        S::ClearType::fill_random(&mut self.data.lambda_clear);

        let lambda_shares_and_macs =
            party.generate_all_parties_shares_vec(&self.data.lambda_clear);
        self.data.lambda_shr = lambda_shares_and_macs.value_shares;
        self.data.lambda_shr_mac = lambda_shares_and_macs.mac_shares;
        eprintln!("  [fake-multiply] generate lambda shares ({} elems) {} ms", size_output, t.elapsed().as_millis());

        // Generate multiplication triples
        let t = Instant::now();
        let mut a_clear = vec![S::ClearType::zero(); size_lhs];
        let mut b_clear = vec![S::ClearType::zero(); size_rhs];
        S::ClearType::fill_random(&mut a_clear);
        S::ClearType::fill_random(&mut b_clear);
        let c_clear =
            matrix_multiply(&a_clear, &b_clear, self.data.dim_row, self.dim_mid, self.data.dim_col);
        eprintln!("  [fake-multiply] matmul for Beaver triple ({}x{}x{}) {} ms", self.data.dim_row, self.dim_mid, self.data.dim_col, t.elapsed().as_millis());

        let t = Instant::now();
        let a_share_with_mac = party.generate_all_parties_shares_vec(&a_clear);
        let b_share_with_mac = party.generate_all_parties_shares_vec(&b_clear);
        let c_share_with_mac = party.generate_all_parties_shares_vec(&c_clear);
        eprintln!("  [fake-multiply] generate a,b,c shares {} ms", t.elapsed().as_millis());

        // delta_x = a - lambda_x, delta_y = b - lambda_y
        let (ix_clear, iy_clear) = {
            let ix = self.data.input_x.as_ref().unwrap().lock().unwrap();
            let iy = self.data.input_y.as_ref().unwrap().lock().unwrap();
            (ix.lambda_clear().clone(), iy.lambda_clear().clone())
        };

        let delta_x_clear = matrix_subtract(&a_clear, &ix_clear);
        let delta_y_clear = matrix_subtract(&b_clear, &iy_clear);

        // Write all data to files
        let t = Instant::now();
        party.write_shares_to_all_parties(&a_share_with_mac.value_shares);
        party.write_shares_to_all_parties(&a_share_with_mac.mac_shares);
        party.write_shares_to_all_parties(&b_share_with_mac.value_shares);
        party.write_shares_to_all_parties(&b_share_with_mac.mac_shares);
        party.write_shares_to_all_parties(&c_share_with_mac.value_shares);
        party.write_shares_to_all_parties(&c_share_with_mac.mac_shares);
        party.write_shares_to_all_parties(&self.data.lambda_shr);
        party.write_shares_to_all_parties(&self.data.lambda_shr_mac);

        party.write_clear_to_all_parties(&delta_x_clear);
        party.write_clear_to_all_parties(&delta_y_clear);
        eprintln!("  [fake-multiply] write to files {} ms", t.elapsed().as_millis());
    }
}

impl<S: Spdz2kShare, const N: usize> FakeGate<S, N> for FakeMultiplyGate<S, N> {
    impl_fake_gate_common!(FakeMultiplyGate, S, N);

    fn do_run_offline(&mut self, party: &mut FakeParty<S, N>) {
        self.do_run_offline_base(party);
    }
}
