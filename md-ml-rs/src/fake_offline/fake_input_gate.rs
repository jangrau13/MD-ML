// By Boshi Yuan (Rust rewrite)

use crate::share::{ShareElement, Spdz2kShare};
use crate::fake_offline::fake_gate::*;
use crate::fake_offline::FakeParty;
use crate::share::widen;

pub struct FakeInputGate<S: Spdz2kShare, const N: usize> {
    pub data: FakeGateData<S, N>,
    owner_id: usize,
}

impl<S: Spdz2kShare, const N: usize> FakeInputGate<S, N> {
    pub fn new(dim_row: usize, dim_col: usize, owner_id: usize) -> Self {
        FakeInputGate {
            data: FakeGateData::new_with_dims(dim_row, dim_col),
            owner_id,
        }
    }
}

impl<S: Spdz2kShare, const N: usize> FakeGate<S, N> for FakeInputGate<S, N> {
    impl_fake_gate_common!(FakeInputGate, S, N);

    fn do_run_offline(&mut self, party: &mut FakeParty<S, N>) {
        let size = self.data.dim_row * self.data.dim_col;

        // lambda values are uniformly random
        self.data.lambda_clear = vec![S::ClearType::zero(); size];
        S::ClearType::fill_random(&mut self.data.lambda_clear);

        // Generate shares
        let shares_and_macs = party.generate_all_parties_shares_vec(&self.data.lambda_clear);
        self.data.lambda_shr = shares_and_macs.value_shares;
        self.data.lambda_shr_mac = shares_and_macs.mac_shares;

        // Write to files (widen lambda_clear to SemiShrType for binary format compatibility)
        let lambda_clear_wide: Vec<S::SemiShrType> = self.data.lambda_clear.iter()
            .map(|&v| widen::<S::ClearType, S::SemiShrType>(v))
            .collect();
        party.write_shares_to_ith_party(&lambda_clear_wide, self.owner_id);
        party.write_shares_to_all_parties(&self.data.lambda_shr);
        party.write_shares_to_all_parties(&self.data.lambda_shr_mac);
    }
}
