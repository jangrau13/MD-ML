// By Boshi Yuan (Rust rewrite)

use crate::share::Spdz2kShare;
use crate::fake_offline::fake_gate::*;
use crate::fake_offline::FakeParty;
use crate::utils::linear_algebra::matrix_add;

pub struct FakeAddGate<S: Spdz2kShare, const N: usize> {
    pub data: FakeGateData<S, N>,
}

impl<S: Spdz2kShare, const N: usize> FakeAddGate<S, N> {
    pub fn new(input_x: FakeGateRef<S, N>, input_y: FakeGateRef<S, N>) -> Self {
        let (dim_row, dim_col) = {
            let gx = input_x.lock().unwrap();
            let gy = input_y.lock().unwrap();
            assert!(
                gx.dim_row() == gy.dim_row() && gx.dim_col() == gy.dim_col(),
                "The inputs of addition gate should have the same dimensions"
            );
            (gx.dim_row(), gx.dim_col())
        };
        let mut data = FakeGateData::new_with_inputs(input_x, Some(input_y));
        data.dim_row = dim_row;
        data.dim_col = dim_col;
        FakeAddGate { data }
    }
}

impl<S: Spdz2kShare, const N: usize> FakeGate<S, N> for FakeAddGate<S, N> {
    impl_fake_gate_common!(FakeAddGate, S, N);

    fn do_run_offline(&mut self, party: &mut FakeParty<S, N>) {
        let (ix_shr, ix_mac, iy_shr, iy_mac) = {
            let ix = self.data.input_x.as_ref().unwrap().lock().unwrap();
            let iy = self.data.input_y.as_ref().unwrap().lock().unwrap();
            (
                ix.lambda_shr().clone(),
                ix.lambda_shr_mac().clone(),
                iy.lambda_shr().clone(),
                iy.lambda_shr_mac().clone(),
            )
        };

        for i in 0..N {
            self.data.lambda_shr[i] = matrix_add(&ix_shr[i], &iy_shr[i]);
            self.data.lambda_shr_mac[i] = matrix_add(&ix_mac[i], &iy_mac[i]);
        }

        party.write_shares_to_all_parties(&self.data.lambda_shr);
        party.write_shares_to_all_parties(&self.data.lambda_shr_mac);
    }
}
