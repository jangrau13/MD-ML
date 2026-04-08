// By Boshi Yuan (Rust rewrite)

use crate::share::Spdz2kShare;
use crate::protocols::gate::*;
use crate::protocols::PartyWithFakeOffline;
use crate::utils::linear_algebra::matrix_subtract;

pub struct SubtractGate<S: Spdz2kShare> {
    pub data: GateData<S>,
}

impl<S: Spdz2kShare> SubtractGate<S> {
    pub fn new(input_x: GateRef<S>, input_y: GateRef<S>) -> Self {
        let (dim_row, dim_col) = {
            let gx = input_x.lock().unwrap();
            let gy = input_y.lock().unwrap();
            assert!(
                gx.dim_row() == gy.dim_row() && gx.dim_col() == gy.dim_col(),
                "The inputs of subtraction gate should have the same dimensions"
            );
            (gx.dim_row(), gx.dim_col())
        };
        let mut data = GateData::new_with_inputs(input_x, Some(input_y));
        data.dim_row = dim_row;
        data.dim_col = dim_col;
        SubtractGate { data }
    }
}

impl<S: Spdz2kShare> Gate<S> for SubtractGate<S> {
    impl_gate_common!(SubtractGate, S);

    fn do_read_offline_from_file(&mut self, _party: &mut PartyWithFakeOffline<S>) {
        // Do nothing
    }

    fn do_run_online(&mut self, _party: &mut PartyWithFakeOffline<S>) {
        let (dx, dy) = {
            let ix = self.data.input_x.as_ref().unwrap().lock().unwrap();
            let iy = self.data.input_y.as_ref().unwrap().lock().unwrap();
            (ix.delta_clear().clone(), iy.delta_clear().clone())
        };
        self.data.delta_clear = matrix_subtract(&dx, &dy);
    }
}
