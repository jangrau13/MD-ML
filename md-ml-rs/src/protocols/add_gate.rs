// By Boshi Yuan (Rust rewrite)

use crate::share::Spdz2kShare;
use crate::protocols::gate::*;
use crate::protocols::PartyWithFakeOffline;
use crate::utils::linear_algebra::matrix_add;

pub struct AddGate<S: Spdz2kShare> {
    pub data: GateData<S>,
}

impl<S: Spdz2kShare> AddGate<S> {
    pub fn new(input_x: GateRef<S>, input_y: GateRef<S>) -> Self {
        let (dim_row, dim_col) = {
            let gx = input_x.lock().unwrap();
            let gy = input_y.lock().unwrap();
            assert!(
                gx.dim_row() == gy.dim_row() && gx.dim_col() == gy.dim_col(),
                "The inputs of addition gate should have the same dimensions"
            );
            (gx.dim_row(), gx.dim_col())
        };
        let mut data = GateData::new_with_inputs(input_x, Some(input_y));
        data.dim_row = dim_row;
        data.dim_col = dim_col;
        AddGate { data }
    }
}

impl<S: Spdz2kShare> Gate<S> for AddGate<S> {
    impl_gate_common!(AddGate, S);

    fn do_read_offline_from_file(&mut self, party: &mut PartyWithFakeOffline<S>) {
        let size = self.data.dim_row * self.data.dim_col;
        self.data.lambda_shr = party.read_shares(size);
        self.data.lambda_shr_mac = party.read_shares(size);
    }

    fn do_run_online(&mut self, _party: &mut PartyWithFakeOffline<S>) {
        let (dx, dy) = {
            let ix = self.data.input_x.as_ref().unwrap().lock().unwrap();
            let iy = self.data.input_y.as_ref().unwrap().lock().unwrap();
            (ix.delta_clear().clone(), iy.delta_clear().clone())
        };
        self.data.delta_clear = matrix_add(&dx, &dy);
    }
}
