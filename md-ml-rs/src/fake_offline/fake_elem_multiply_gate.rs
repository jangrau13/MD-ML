// By Boshi Yuan (Rust rewrite)

use crate::share::{ShareElement, Spdz2kShare};
use crate::fake_offline::fake_gate::*;
use crate::fake_offline::FakeParty;
use crate::utils::linear_algebra::*;

pub struct FakeElemMultiplyGate<S: Spdz2kShare, const N: usize> {
    pub data: FakeGateData<S, N>,
}

impl<S: Spdz2kShare, const N: usize> FakeElemMultiplyGate<S, N> {
    pub fn new(input_x: FakeGateRef<S, N>, input_y: FakeGateRef<S, N>) -> Self {
        let (dim_row, dim_col) = {
            let gx = input_x.lock().unwrap();
            let gy = input_y.lock().unwrap();
            assert!(
                gx.dim_row() == gy.dim_row() && gx.dim_col() == gy.dim_col(),
                "The inputs of element-wise multiplication gate should have compatible dimensions"
            );
            (gx.dim_row(), gx.dim_col())
        };
        let mut data = FakeGateData::new_with_inputs(input_x, Some(input_y));
        data.dim_row = dim_row;
        data.dim_col = dim_col;
        FakeElemMultiplyGate { data }
    }
}

impl<S: Spdz2kShare, const N: usize> FakeGate<S, N> for FakeElemMultiplyGate<S, N> {
    impl_fake_gate_common!(FakeElemMultiplyGate, S, N);

    fn do_run_offline(&mut self, party: &mut FakeParty<S, N>) {
        let size = self.data.dim_row * self.data.dim_col;

        self.data.lambda_clear = vec![S::ClearType::zero(); size];
        S::ClearType::fill_random(&mut self.data.lambda_clear);

        let lambda_shares_and_macs =
            party.generate_all_parties_shares_vec(&self.data.lambda_clear);
        self.data.lambda_shr = lambda_shares_and_macs.value_shares;
        self.data.lambda_shr_mac = lambda_shares_and_macs.mac_shares;

        let mut a_clear = vec![S::ClearType::zero(); size];
        let mut b_clear = vec![S::ClearType::zero(); size];
        S::ClearType::fill_random(&mut a_clear);
        S::ClearType::fill_random(&mut b_clear);
        let c_clear = matrix_elem_multiply(&a_clear, &b_clear);

        let a_share = party.generate_all_parties_shares_vec(&a_clear);
        let b_share = party.generate_all_parties_shares_vec(&b_clear);
        let c_share = party.generate_all_parties_shares_vec(&c_clear);

        let (ix_clear, iy_clear) = {
            let ix = self.data.input_x.as_ref().unwrap().lock().unwrap();
            let iy = self.data.input_y.as_ref().unwrap().lock().unwrap();
            (ix.lambda_clear().clone(), iy.lambda_clear().clone())
        };

        let delta_x_clear = matrix_subtract(&a_clear, &ix_clear);
        let delta_y_clear = matrix_subtract(&b_clear, &iy_clear);

        party.write_shares_to_all_parties(&a_share.value_shares);
        party.write_shares_to_all_parties(&a_share.mac_shares);
        party.write_shares_to_all_parties(&b_share.value_shares);
        party.write_shares_to_all_parties(&b_share.mac_shares);
        party.write_shares_to_all_parties(&c_share.value_shares);
        party.write_shares_to_all_parties(&c_share.mac_shares);
        party.write_shares_to_all_parties(&self.data.lambda_shr);
        party.write_shares_to_all_parties(&self.data.lambda_shr_mac);

        party.write_clear_to_all_parties(&delta_x_clear);
        party.write_clear_to_all_parties(&delta_y_clear);
    }
}
