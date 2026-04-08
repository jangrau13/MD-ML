// By Boshi Yuan (Rust rewrite)

use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::share::Spdz2kShare;
use crate::fake_offline::FakeParty;

pub type FakeGateRef<S, const N: usize> = Arc<Mutex<dyn FakeGate<S, N>>>;

pub trait FakeGate<S: Spdz2kShare, const N: usize>: Send {
    fn name(&self) -> &'static str;
    fn dim_row(&self) -> usize;
    fn dim_col(&self) -> usize;

    fn input_x(&self) -> Option<FakeGateRef<S, N>>;
    fn input_y(&self) -> Option<FakeGateRef<S, N>>;

    fn lambda_clear(&self) -> &Vec<S::ClearType>;
    fn lambda_clear_mut(&mut self) -> &mut Vec<S::ClearType>;

    fn lambda_shr(&self) -> &[Vec<S::SemiShrType>; N];
    fn lambda_shr_mut(&mut self) -> &mut [Vec<S::SemiShrType>; N];

    fn lambda_shr_mac(&self) -> &[Vec<S::SemiShrType>; N];
    fn lambda_shr_mac_mut(&mut self) -> &mut [Vec<S::SemiShrType>; N];

    fn is_evaluated_offline(&self) -> bool;
    fn set_evaluated_offline(&mut self);

    fn do_run_offline(&mut self, party: &mut FakeParty<S, N>);
}

pub struct FakeGateData<S: Spdz2kShare, const N: usize> {
    pub dim_row: usize,
    pub dim_col: usize,
    pub lambda_clear: Vec<S::ClearType>,
    pub lambda_shr: [Vec<S::SemiShrType>; N],
    pub lambda_shr_mac: [Vec<S::SemiShrType>; N],
    pub input_x: Option<FakeGateRef<S, N>>,
    pub input_y: Option<FakeGateRef<S, N>>,
    pub evaluated_offline: bool,
}

impl<S: Spdz2kShare, const N: usize> FakeGateData<S, N> {
    pub fn new_with_dims(dim_row: usize, dim_col: usize) -> Self {
        FakeGateData {
            dim_row,
            dim_col,
            lambda_clear: Vec::new(),
            lambda_shr: std::array::from_fn(|_| Vec::new()),
            lambda_shr_mac: std::array::from_fn(|_| Vec::new()),
            input_x: None,
            input_y: None,
            evaluated_offline: false,
        }
    }

    pub fn new_with_inputs(
        input_x: FakeGateRef<S, N>,
        input_y: Option<FakeGateRef<S, N>>,
    ) -> Self {
        FakeGateData {
            dim_row: 1,
            dim_col: 1,
            lambda_clear: Vec::new(),
            lambda_shr: std::array::from_fn(|_| Vec::new()),
            lambda_shr_mac: std::array::from_fn(|_| Vec::new()),
            input_x: Some(input_x),
            input_y,
            evaluated_offline: false,
        }
    }
}

macro_rules! impl_fake_gate_common {
    ($struct_name:ident, $shr:ident, $n:expr) => {
        fn name(&self) -> &'static str { stringify!($struct_name) }
        fn dim_row(&self) -> usize { self.data.dim_row }
        fn dim_col(&self) -> usize { self.data.dim_col }

        fn input_x(&self) -> Option<FakeGateRef<$shr, $n>> { self.data.input_x.clone() }
        fn input_y(&self) -> Option<FakeGateRef<$shr, $n>> { self.data.input_y.clone() }

        fn lambda_clear(&self) -> &Vec<$shr::ClearType> { &self.data.lambda_clear }
        fn lambda_clear_mut(&mut self) -> &mut Vec<$shr::ClearType> { &mut self.data.lambda_clear }

        fn lambda_shr(&self) -> &[Vec<$shr::SemiShrType>; $n] { &self.data.lambda_shr }
        fn lambda_shr_mut(&mut self) -> &mut [Vec<$shr::SemiShrType>; $n] { &mut self.data.lambda_shr }

        fn lambda_shr_mac(&self) -> &[Vec<$shr::SemiShrType>; $n] { &self.data.lambda_shr_mac }
        fn lambda_shr_mac_mut(&mut self) -> &mut [Vec<$shr::SemiShrType>; $n] { &mut self.data.lambda_shr_mac }

        fn is_evaluated_offline(&self) -> bool { self.data.evaluated_offline }
        fn set_evaluated_offline(&mut self) { self.data.evaluated_offline = true; }
    };
}

pub(crate) use impl_fake_gate_common;

/// Recursively run offline phase for a fake gate and its inputs
pub fn run_fake_offline<S: Spdz2kShare, const N: usize>(
    gate: &FakeGateRef<S, N>,
    party: &mut FakeParty<S, N>,
) {
    let (input_x, input_y, already_done) = {
        let g = gate.lock().unwrap();
        (g.input_x(), g.input_y(), g.is_evaluated_offline())
    };

    if already_done {
        return;
    }

    if let Some(ref ix) = input_x {
        run_fake_offline(ix, party);
    }
    if let Some(ref iy) = input_y {
        run_fake_offline(iy, party);
    }

    let mut g = gate.lock().unwrap();
    if !g.is_evaluated_offline() {
        let name = g.name();
        let dims = format!("{}x{}", g.dim_row(), g.dim_col());
        eprintln!("[fake-offline] Generating {} ({}) ...", name, dims);
        let t = Instant::now();
        g.do_run_offline(party);
        g.set_evaluated_offline();
        eprintln!("[fake-offline] Generating {} ({}) done in {} ms", name, dims, t.elapsed().as_millis());
    }
}
