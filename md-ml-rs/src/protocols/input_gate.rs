// By Boshi Yuan (Rust rewrite)

use crate::share::{ShareElement, Spdz2kShare};
use crate::protocols::gate::*;
use crate::protocols::PartyWithFakeOffline;
use crate::utils::linear_algebra::matrix_add;

pub struct InputGate<S: Spdz2kShare> {
    pub data: GateData<S>,
    owner_id: usize,
    lambda_clear: Vec<S::SemiShrType>,
    input_value: Vec<S::SemiShrType>,
}

impl<S: Spdz2kShare> InputGate<S> {
    pub fn new(dim_row: usize, dim_col: usize, owner_id: usize) -> Self {
        InputGate {
            data: GateData::new_with_dims(dim_row, dim_col),
            owner_id,
            lambda_clear: Vec::new(),
            input_value: Vec::new(),
        }
    }

    pub fn set_input(&mut self, input_value: &[S::ClearType], my_id: usize) {
        assert_eq!(
            my_id, self.owner_id,
            "Not the owner of input gate, cannot set input"
        );
        assert_eq!(
            input_value.len(),
            self.data.dim_row * self.data.dim_col,
            "Input vector and gate don't match in size"
        );
        // Convert ClearType -> SemiShrType (widening)
        self.input_value = input_value
            .iter()
            .map(|&v| {
                // Convert through bytes: ClearType -> bytes -> SemiShrType
                let bytes = v.to_le_bytes_vec();
                let mut extended = vec![0u8; S::SemiShrType::byte_size()];
                extended[..bytes.len()].copy_from_slice(&bytes);
                S::SemiShrType::from_le_bytes(&extended)
            })
            .collect();
    }
}

impl<S: Spdz2kShare> Gate<S> for InputGate<S> {
    impl_gate_common!(InputGate, S);

    fn do_read_offline_from_file(&mut self, party: &mut PartyWithFakeOffline<S>) {
        let size = self.data.dim_row * self.data.dim_col;

        if party.my_id() == self.owner_id {
            self.lambda_clear = party.read_shares(size);
        }

        self.data.lambda_shr = party.read_shares(size);
        self.data.lambda_shr_mac = party.read_shares(size);
    }

    fn do_run_online(&mut self, party: &mut PartyWithFakeOffline<S>) {
        if party.my_id() == self.owner_id {
            self.data.delta_clear = matrix_add(&self.input_value, &self.lambda_clear);
            party.party.send_vec_to_other(&self.data.delta_clear);
        } else {
            let size = self.data.dim_row * self.data.dim_col;
            self.data.delta_clear = party.party.receive_vec_from_other(size);
        }
    }
}
