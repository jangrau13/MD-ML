// By Boshi Yuan (Rust rewrite)

use crate::share::Spdz2kShare;
use crate::fake_offline::fake_gate::*;
use crate::fake_offline::FakeParty;

pub struct FakeOutputGate<S: Spdz2kShare, const N: usize> {
    pub data: FakeGateData<S, N>,
}

impl<S: Spdz2kShare, const N: usize> FakeOutputGate<S, N> {
    pub fn new(input_x: FakeGateRef<S, N>) -> Self {
        let (dim_row, dim_col) = {
            let g = input_x.lock().unwrap();
            (g.dim_row(), g.dim_col())
        };
        let mut data = FakeGateData::new_with_inputs(input_x, None);
        data.dim_row = dim_row;
        data.dim_col = dim_col;
        FakeOutputGate { data }
    }
}

impl<S: Spdz2kShare, const N: usize> FakeGate<S, N> for FakeOutputGate<S, N> {
    impl_fake_gate_common!(FakeOutputGate, S, N);

    fn do_run_offline(&mut self, _party: &mut FakeParty<S, N>) {
        // Do nothing
    }
}
