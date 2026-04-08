// By Boshi Yuan (Rust rewrite)

use crate::share::Spdz2kShare;

use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Type alias for a shared gate reference
pub type GateRef<S> = Arc<Mutex<dyn Gate<S>>>;

/// The Gate trait - base for all computation gates in the protocol.
///
/// Each gate holds a matrix (flat vector) of shares plus pointers to input gates,
/// forming a DAG-based circuit.
pub trait Gate<S: Spdz2kShare>: Send {
    fn name(&self) -> &'static str;
    fn dim_row(&self) -> usize;
    fn dim_col(&self) -> usize;

    fn lambda_shr(&self) -> &Vec<S::SemiShrType>;
    fn lambda_shr_mut(&mut self) -> &mut Vec<S::SemiShrType>;

    fn lambda_shr_mac(&self) -> &Vec<S::SemiShrType>;
    fn lambda_shr_mac_mut(&mut self) -> &mut Vec<S::SemiShrType>;

    fn delta_clear(&self) -> &Vec<S::SemiShrType>;
    fn delta_clear_mut(&mut self) -> &mut Vec<S::SemiShrType>;

    fn input_x(&self) -> Option<GateRef<S>>;
    fn input_y(&self) -> Option<GateRef<S>>;

    fn is_read_offline(&self) -> bool;
    fn set_read_offline(&mut self);

    fn is_evaluated_online(&self) -> bool;
    fn set_evaluated_online(&mut self);

    fn do_read_offline_from_file(&mut self, party: &mut crate::protocols::PartyWithFakeOffline<S>);
    fn do_run_online(&mut self, party: &mut crate::protocols::PartyWithFakeOffline<S>);
}

/// Common gate data that all gate structs embed
pub struct GateData<S: Spdz2kShare> {
    pub dim_row: usize,
    pub dim_col: usize,
    pub lambda_shr: Vec<S::SemiShrType>,
    pub lambda_shr_mac: Vec<S::SemiShrType>,
    pub delta_clear: Vec<S::SemiShrType>,
    pub input_x: Option<GateRef<S>>,
    pub input_y: Option<GateRef<S>>,
    pub read_offline: bool,
    pub evaluated_online: bool,
}

impl<S: Spdz2kShare> GateData<S> {
    pub fn new_with_dims(dim_row: usize, dim_col: usize) -> Self {
        GateData {
            dim_row,
            dim_col,
            lambda_shr: Vec::new(),
            lambda_shr_mac: Vec::new(),
            delta_clear: Vec::new(),
            input_x: None,
            input_y: None,
            read_offline: false,
            evaluated_online: false,
        }
    }

    pub fn new_with_inputs(input_x: GateRef<S>, input_y: Option<GateRef<S>>) -> Self {
        GateData {
            dim_row: 1,
            dim_col: 1,
            lambda_shr: Vec::new(),
            lambda_shr_mac: Vec::new(),
            delta_clear: Vec::new(),
            input_x: Some(input_x),
            input_y,
            read_offline: false,
            evaluated_online: false,
        }
    }
}

/// Macro to implement the common Gate trait methods that delegate to GateData
macro_rules! impl_gate_common {
    ($struct_name:ident, $shr:ident) => {
        fn name(&self) -> &'static str { stringify!($struct_name) }
        fn dim_row(&self) -> usize { self.data.dim_row }
        fn dim_col(&self) -> usize { self.data.dim_col }

        fn lambda_shr(&self) -> &Vec<$shr::SemiShrType> { &self.data.lambda_shr }
        fn lambda_shr_mut(&mut self) -> &mut Vec<$shr::SemiShrType> { &mut self.data.lambda_shr }

        fn lambda_shr_mac(&self) -> &Vec<$shr::SemiShrType> { &self.data.lambda_shr_mac }
        fn lambda_shr_mac_mut(&mut self) -> &mut Vec<$shr::SemiShrType> { &mut self.data.lambda_shr_mac }

        fn delta_clear(&self) -> &Vec<$shr::SemiShrType> { &self.data.delta_clear }
        fn delta_clear_mut(&mut self) -> &mut Vec<$shr::SemiShrType> { &mut self.data.delta_clear }

        fn input_x(&self) -> Option<GateRef<$shr>> { self.data.input_x.clone() }
        fn input_y(&self) -> Option<GateRef<$shr>> { self.data.input_y.clone() }

        fn is_read_offline(&self) -> bool { self.data.read_offline }
        fn set_read_offline(&mut self) { self.data.read_offline = true; }

        fn is_evaluated_online(&self) -> bool { self.data.evaluated_online }
        fn set_evaluated_online(&mut self) { self.data.evaluated_online = true; }
    };
}

pub(crate) use impl_gate_common;

/// Recursively read offline data from file for a gate and its inputs
pub fn read_offline_from_file<S: Spdz2kShare>(
    gate: &GateRef<S>,
    party: &mut crate::protocols::PartyWithFakeOffline<S>,
) {
    let (input_x, input_y, already_read) = {
        let g = gate.lock().unwrap();
        (g.input_x(), g.input_y(), g.is_read_offline())
    };

    if already_read {
        return;
    }

    if let Some(ref ix) = input_x {
        read_offline_from_file(ix, party);
    }
    if let Some(ref iy) = input_y {
        read_offline_from_file(iy, party);
    }

    let mut g = gate.lock().unwrap();
    if !g.is_read_offline() {
        let name = g.name();
        let dims = format!("{}x{}", g.dim_row(), g.dim_col());
        eprintln!("[offline] Reading {} ({}) ...", name, dims);
        let t = Instant::now();
        g.do_read_offline_from_file(party);
        g.set_read_offline();
        eprintln!("[offline] Reading {} ({}) done in {} ms", name, dims, t.elapsed().as_millis());
    }
}

/// Recursively run the online phase for a gate and its inputs
pub fn run_online<S: Spdz2kShare>(
    gate: &GateRef<S>,
    party: &mut crate::protocols::PartyWithFakeOffline<S>,
) {
    let (input_x, input_y, already_evaluated) = {
        let g = gate.lock().unwrap();
        (g.input_x(), g.input_y(), g.is_evaluated_online())
    };

    if already_evaluated {
        return;
    }

    if let Some(ref ix) = input_x {
        run_online(ix, party);
    }
    if let Some(ref iy) = input_y {
        run_online(iy, party);
    }

    let mut g = gate.lock().unwrap();
    if !g.is_evaluated_online() {
        let name = g.name();
        let dims = format!("{}x{}", g.dim_row(), g.dim_col());
        eprintln!("[online]  Running {} ({}) ...", name, dims);
        let t = Instant::now();
        g.do_run_online(party);
        g.set_evaluated_online();
        eprintln!("[online]  Running {} ({}) done in {} ms", name, dims, t.elapsed().as_millis());
    }
}
