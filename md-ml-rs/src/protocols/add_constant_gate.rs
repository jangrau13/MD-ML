// By Boshi Yuan (Rust rewrite)

use crate::share::{ShareElement, Spdz2kShare};
use crate::protocols::gate::*;
use crate::protocols::PartyWithFakeOffline;
use crate::utils::linear_algebra::matrix_add_constant;

pub struct AddConstantGate<S: Spdz2kShare> {
    pub data: GateData<S>,
    constant: S::ClearType,
}

impl<S: Spdz2kShare> AddConstantGate<S> {
    pub fn new(input_x: GateRef<S>, constant: S::ClearType) -> Self {
        let (dim_row, dim_col) = {
            let g = input_x.lock().unwrap();
            (g.dim_row(), g.dim_col())
        };
        let mut data = GateData::new_with_inputs(input_x, None);
        data.dim_row = dim_row;
        data.dim_col = dim_col;
        AddConstantGate { data, constant }
    }
}

impl<S: Spdz2kShare> Gate<S> for AddConstantGate<S> {
    impl_gate_common!(AddConstantGate, S);

    fn do_read_offline_from_file(&mut self, _party: &mut PartyWithFakeOffline<S>) {
        let ix = self.data.input_x.as_ref().unwrap().lock().unwrap();
        self.data.lambda_shr = ix.lambda_shr().clone();
        self.data.lambda_shr_mac = ix.lambda_shr_mac().clone();
    }

    fn do_run_online(&mut self, _party: &mut PartyWithFakeOffline<S>) {
        let dx = {
            let ix = self.data.input_x.as_ref().unwrap().lock().unwrap();
            ix.delta_clear().clone()
        };
        // Widen constant to SemiShrType
        let bytes = self.constant.to_le_bytes_vec();
        let mut extended = vec![0u8; S::SemiShrType::byte_size()];
        extended[..bytes.len()].copy_from_slice(&bytes);
        let constant_wide = S::SemiShrType::from_le_bytes(&extended);

        self.data.delta_clear = matrix_add_constant(&dx, constant_wide);
    }
}
