// By Boshi Yuan (Rust rewrite)

use crate::share::Spdz2kShare;
use crate::fake_offline::fake_gate::*;
use crate::fake_offline::fake_multiply_gate::FakeMultiplyGate;
use crate::fake_offline::FakeParty;
use crate::utils::fixed_point::truncate_clear_vec;

pub struct FakeMultiplyTruncGate<S: Spdz2kShare, const N: usize> {
    inner: FakeMultiplyGate<S, N>,
}

impl<S: Spdz2kShare, const N: usize> FakeMultiplyTruncGate<S, N> {
    pub fn new(input_x: FakeGateRef<S, N>, input_y: FakeGateRef<S, N>) -> Self {
        let inner = FakeMultiplyGate::new(input_x, input_y);
        FakeMultiplyTruncGate { inner }
    }
}

impl<S: Spdz2kShare, const N: usize> FakeGate<S, N> for FakeMultiplyTruncGate<S, N> {
    fn name(&self) -> &'static str { "FakeMultiplyTruncGate" }
    fn dim_row(&self) -> usize { self.inner.data.dim_row }
    fn dim_col(&self) -> usize { self.inner.data.dim_col }

    fn input_x(&self) -> Option<FakeGateRef<S, N>> { self.inner.data.input_x.clone() }
    fn input_y(&self) -> Option<FakeGateRef<S, N>> { self.inner.data.input_y.clone() }

    fn lambda_clear(&self) -> &Vec<S::ClearType> { &self.inner.data.lambda_clear }
    fn lambda_clear_mut(&mut self) -> &mut Vec<S::ClearType> { &mut self.inner.data.lambda_clear }

    fn lambda_shr(&self) -> &[Vec<S::SemiShrType>; N] { &self.inner.data.lambda_shr }
    fn lambda_shr_mut(&mut self) -> &mut [Vec<S::SemiShrType>; N] { &mut self.inner.data.lambda_shr }

    fn lambda_shr_mac(&self) -> &[Vec<S::SemiShrType>; N] { &self.inner.data.lambda_shr_mac }
    fn lambda_shr_mac_mut(&mut self) -> &mut [Vec<S::SemiShrType>; N] { &mut self.inner.data.lambda_shr_mac }

    fn is_evaluated_offline(&self) -> bool { self.inner.data.evaluated_offline }
    fn set_evaluated_offline(&mut self) { self.inner.data.evaluated_offline = true; }

    fn do_run_offline(&mut self, party: &mut FakeParty<S, N>) {
        self.inner.do_run_offline_base(party);

        // Compute truncated lambda as the "real" lambda
        let lambda_prime_clear = truncate_clear_vec(&self.inner.data.lambda_clear);
        let lambda_prime_share_with_mac =
            party.generate_all_parties_shares_vec(&lambda_prime_clear);

        let lambda_prime_shr = lambda_prime_share_with_mac.value_shares;
        let lambda_prime_shr_mac = lambda_prime_share_with_mac.mac_shares;

        // Write to files
        party.write_shares_to_all_parties(&lambda_prime_shr);
        party.write_shares_to_all_parties(&lambda_prime_shr_mac);

        // Swap: the truncated lambda becomes the "real" lambda
        self.inner.data.lambda_clear = lambda_prime_clear;
        self.inner.data.lambda_shr = lambda_prime_shr;
        self.inner.data.lambda_shr_mac = lambda_prime_shr_mac;
    }
}
