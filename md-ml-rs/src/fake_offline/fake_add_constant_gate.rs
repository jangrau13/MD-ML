// By Boshi Yuan (Rust rewrite)

use crate::share::Spdz2kShare;
use crate::fake_offline::fake_gate::*;
use crate::fake_offline::FakeParty;

pub struct FakeAddConstantGate<S: Spdz2kShare, const N: usize> {
    pub data: FakeGateData<S, N>,
    _constant: S::ClearType,
}

impl<S: Spdz2kShare, const N: usize> FakeAddConstantGate<S, N> {
    pub fn new(input_x: FakeGateRef<S, N>, constant: S::ClearType) -> Self {
        let (dim_row, dim_col) = {
            let g = input_x.lock().unwrap();
            (g.dim_row(), g.dim_col())
        };
        let mut data = FakeGateData::new_with_inputs(input_x, None);
        data.dim_row = dim_row;
        data.dim_col = dim_col;
        FakeAddConstantGate {
            data,
            _constant: constant,
        }
    }
}

impl<S: Spdz2kShare, const N: usize> FakeGate<S, N> for FakeAddConstantGate<S, N> {
    impl_fake_gate_common!(FakeAddConstantGate, S, N);

    fn do_run_offline(&mut self, _party: &mut FakeParty<S, N>) {
        let (ix_clear, ix_shr, ix_mac) = {
            let ix = self.data.input_x.as_ref().unwrap().lock().unwrap();
            (
                ix.lambda_clear().clone(),
                ix.lambda_shr().clone(),
                ix.lambda_shr_mac().clone(),
            )
        };

        self.data.lambda_clear = ix_clear;
        self.data.lambda_shr = ix_shr;
        self.data.lambda_shr_mac = ix_mac;
    }
}
