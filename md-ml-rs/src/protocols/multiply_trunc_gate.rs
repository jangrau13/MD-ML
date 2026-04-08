// By Boshi Yuan (Rust rewrite)

use crate::share::Spdz2kShare;
use crate::protocols::gate::*;
use crate::protocols::PartyWithFakeOffline;
use crate::protocols::multiply_gate::MultiplyGate;
use crate::utils::fixed_point::truncate_clear_vec_inplace;

pub struct MultiplyTruncGate<S: Spdz2kShare> {
    inner: MultiplyGate<S>,
    lambda_prime_shr: Vec<S::SemiShrType>,
    lambda_prime_shr_mac: Vec<S::SemiShrType>,
}

impl<S: Spdz2kShare> MultiplyTruncGate<S> {
    pub fn new(input_x: GateRef<S>, input_y: GateRef<S>) -> Self {
        let inner = MultiplyGate::new(input_x, input_y);
        MultiplyTruncGate {
            inner,
            lambda_prime_shr: Vec::new(),
            lambda_prime_shr_mac: Vec::new(),
        }
    }
}

impl<S: Spdz2kShare> Gate<S> for MultiplyTruncGate<S> {
    fn name(&self) -> &'static str { "MultiplyTruncGate" }
    fn dim_row(&self) -> usize { self.inner.data.dim_row }
    fn dim_col(&self) -> usize { self.inner.data.dim_col }

    fn lambda_shr(&self) -> &Vec<S::SemiShrType> { &self.inner.data.lambda_shr }
    fn lambda_shr_mut(&mut self) -> &mut Vec<S::SemiShrType> { &mut self.inner.data.lambda_shr }

    fn lambda_shr_mac(&self) -> &Vec<S::SemiShrType> { &self.inner.data.lambda_shr_mac }
    fn lambda_shr_mac_mut(&mut self) -> &mut Vec<S::SemiShrType> { &mut self.inner.data.lambda_shr_mac }

    fn delta_clear(&self) -> &Vec<S::SemiShrType> { &self.inner.data.delta_clear }
    fn delta_clear_mut(&mut self) -> &mut Vec<S::SemiShrType> { &mut self.inner.data.delta_clear }

    fn input_x(&self) -> Option<GateRef<S>> { self.inner.data.input_x.clone() }
    fn input_y(&self) -> Option<GateRef<S>> { self.inner.data.input_y.clone() }

    fn is_read_offline(&self) -> bool { self.inner.data.read_offline }
    fn set_read_offline(&mut self) { self.inner.data.read_offline = true; }

    fn is_evaluated_online(&self) -> bool { self.inner.data.evaluated_online }
    fn set_evaluated_online(&mut self) { self.inner.data.evaluated_online = true; }

    fn do_read_offline_from_file(&mut self, party: &mut PartyWithFakeOffline<S>) {
        self.inner.do_read_offline_base(party);
        let size_output = self.inner.data.dim_row * self.inner.data.dim_col;
        self.lambda_prime_shr = party.read_shares(size_output);
        self.lambda_prime_shr_mac = party.read_shares(size_output);
    }

    fn do_run_online(&mut self, party: &mut PartyWithFakeOffline<S>) {
        self.inner.do_run_online_base(party);

        // Swap lambda_shr with lambda_prime_shr
        std::mem::swap(&mut self.lambda_prime_shr, &mut self.inner.data.lambda_shr);
        std::mem::swap(&mut self.lambda_prime_shr_mac, &mut self.inner.data.lambda_shr_mac);

        // Truncate Delta_z
        truncate_clear_vec_inplace(&mut self.inner.data.delta_clear);
    }
}
