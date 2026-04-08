// By Boshi Yuan (Rust rewrite)

use crate::share::{ShareElement, Spdz2kShare};
use crate::share::widen;
use crate::protocols::gate::*;
use crate::protocols::PartyWithFakeOffline;
use crate::utils::linear_algebra::*;
use std::io::{Read, Write};
use std::thread;
use std::time::Instant;

pub struct MultiplyGate<S: Spdz2kShare> {
    pub data: GateData<S>,
    pub dim_mid: usize,
    pub a_shr: Vec<S::SemiShrType>,
    a_shr_mac: Vec<S::SemiShrType>,
    pub b_shr: Vec<S::SemiShrType>,
    b_shr_mac: Vec<S::SemiShrType>,
    pub c_shr: Vec<S::SemiShrType>,
    c_shr_mac: Vec<S::SemiShrType>,
    pub delta_x_clear: Vec<S::SemiShrType>,
    pub delta_y_clear: Vec<S::SemiShrType>,
}

impl<S: Spdz2kShare> MultiplyGate<S> {
    pub fn new(input_x: GateRef<S>, input_y: GateRef<S>) -> Self {
        let (dim_row, dim_mid, dim_col) = {
            let gx = input_x.lock().unwrap();
            let gy = input_y.lock().unwrap();
            assert!(
                gx.dim_col() == gy.dim_row(),
                "The inputs of multiplication gate should have compatible dimensions"
            );
            (gx.dim_row(), gx.dim_col(), gy.dim_col())
        };
        let mut data = GateData::new_with_inputs(input_x, Some(input_y));
        data.dim_row = dim_row;
        data.dim_col = dim_col;
        MultiplyGate {
            data,
            dim_mid,
            a_shr: Vec::new(),
            a_shr_mac: Vec::new(),
            b_shr: Vec::new(),
            b_shr_mac: Vec::new(),
            c_shr: Vec::new(),
            c_shr_mac: Vec::new(),
            delta_x_clear: Vec::new(),
            delta_y_clear: Vec::new(),
        }
    }

    /// Read offline data - separated so MultiplyTruncGate can call it
    pub fn do_read_offline_base(&mut self, party: &mut PartyWithFakeOffline<S>) {
        let size_lhs = self.data.dim_row * self.dim_mid;
        let size_rhs = self.dim_mid * self.data.dim_col;
        let size_output = self.data.dim_row * self.data.dim_col;

        self.a_shr = party.read_shares(size_lhs);
        self.a_shr_mac = party.read_shares(size_lhs);
        self.b_shr = party.read_shares(size_rhs);
        self.b_shr_mac = party.read_shares(size_rhs);
        self.c_shr = party.read_shares(size_output);
        self.c_shr_mac = party.read_shares(size_output);
        self.data.lambda_shr = party.read_shares(size_output);
        self.data.lambda_shr_mac = party.read_shares(size_output);
        self.delta_x_clear = party.read_shares(size_lhs);
        self.delta_y_clear = party.read_shares(size_rhs);
    }

    /// Run online phase - separated so MultiplyTruncGate can call it
    pub fn do_run_online_base(&mut self, party: &mut PartyWithFakeOffline<S>) {
        let dim_row = self.data.dim_row;
        let dim_mid = self.dim_mid;
        let dim_col = self.data.dim_col;
        let my_id = party.my_id();
        let global_key_shr = party.global_key_shr();

        // Get Delta_x and Delta_y from inputs
        let (delta_x, delta_y) = {
            let ix = self.data.input_x.as_ref().unwrap().lock().unwrap();
            let iy = self.data.input_y.as_ref().unwrap().lock().unwrap();
            (ix.delta_clear().clone(), iy.delta_clear().clone())
        };

        // temp_x = Delta_x + delta_x
        let temp_x = matrix_add(&delta_x, &self.delta_x_clear);
        // temp_y = Delta_y + delta_y
        let temp_y = matrix_add(&delta_y, &self.delta_y_clear);

        // temp_xy = temp_x * temp_y
        let t = Instant::now();
        let temp_xy = matrix_multiply(&temp_x, &temp_y, dim_row, dim_mid, dim_col);
        eprintln!("  [multiply] matmul 1/5 (temp_x * temp_y) {} ms", t.elapsed().as_millis());

        // Compute [Delta_z]
        // [Delta_z] = [c] + [lambda_z]
        let mut delta_z_shr = matrix_add(&self.c_shr, &self.data.lambda_shr);

        // [Delta_z] -= [a] * temp_y
        let t = Instant::now();
        matrix_subtract_assign(
            &mut delta_z_shr,
            &matrix_multiply(&self.a_shr, &temp_y, dim_row, dim_mid, dim_col),
        );
        eprintln!("  [multiply] matmul 2/5 (a * temp_y) {} ms", t.elapsed().as_millis());

        // [Delta_z] -= temp_x * [b]
        let t = Instant::now();
        matrix_subtract_assign(
            &mut delta_z_shr,
            &matrix_multiply(&temp_x, &self.b_shr, dim_row, dim_mid, dim_col),
        );
        eprintln!("  [multiply] matmul 3/5 (temp_x * b) {} ms", t.elapsed().as_millis());

        if my_id == 0 {
            matrix_add_assign(&mut delta_z_shr, &temp_xy);
        }

        // Compute Delta_z_mac
        let global_key_wide = widen::<S::GlobalKeyType, S::SemiShrType>(global_key_shr);
        let mut delta_z_mac = matrix_scalar(&temp_xy, global_key_wide);
        // Fuse c_shr_mac + lambda_shr_mac into delta_z_mac without intermediate allocation
        matrix_add_assign(&mut delta_z_mac, &self.c_shr_mac);
        matrix_add_assign(&mut delta_z_mac, &self.data.lambda_shr_mac);

        let t = Instant::now();
        matrix_subtract_assign(
            &mut delta_z_mac,
            &matrix_multiply(&self.a_shr_mac, &temp_y, dim_row, dim_mid, dim_col),
        );
        eprintln!("  [multiply] matmul 4/5 (a_mac * temp_y) {} ms", t.elapsed().as_millis());

        let t = Instant::now();
        matrix_subtract_assign(
            &mut delta_z_mac,
            &matrix_multiply(&temp_x, &self.b_shr_mac, dim_row, dim_mid, dim_col),
        );
        eprintln!("  [multiply] matmul 5/5 (temp_x * b_mac) {} ms", t.elapsed().as_millis());

        // Exchange Delta_z_shr with other party (send/receive concurrently)
        let t = Instant::now();
        let total_bytes = (dim_row * dim_col) * S::SemiShrType::byte_size();

        // Zero-copy serialization: view the element slice directly as bytes
        let send_bytes = S::SemiShrType::slice_as_bytes(&delta_z_shr).to_vec();

        let other_id = 1 - my_id;
        let (send_stream, recv_stream) = party.party.split_streams(other_id);
        let mut send_clone = send_stream.try_clone().expect("Failed to clone");
        let mut recv_clone = recv_stream.try_clone().expect("Failed to clone");

        let t1 = thread::spawn(move || {
            // Write in chunks to avoid OS "No buffer space available" error
            const CHUNK: usize = 1 << 20; // 1 MB
            for chunk in send_bytes.chunks(CHUNK) {
                send_clone.write_all(chunk).expect("Failed to send");
            }
        });

        let t2 = thread::spawn(move || -> Vec<u8> {
            let mut buf = vec![0u8; total_bytes];
            recv_clone.read_exact(&mut buf).expect("Failed to receive");
            buf
        });

        t1.join().expect("Send thread panicked");
        let recv_buf = t2.join().expect("Recv thread panicked");
        party.party.add_bytes_sent(total_bytes as u64);
        eprintln!("  [multiply] network exchange {} ms", t.elapsed().as_millis());

        // Zero-copy deserialization
        self.data.delta_clear = S::SemiShrType::vec_from_bytes(&recv_buf);

        matrix_add_assign(&mut self.data.delta_clear, &delta_z_shr);

        // Remove upper bits
        S::remove_upper_bits_inplace(&mut self.data.delta_clear);

        // Free preprocessing data
        self.a_shr = Vec::new();
        self.a_shr_mac = Vec::new();
        self.b_shr = Vec::new();
        self.b_shr_mac = Vec::new();
        self.c_shr = Vec::new();
        self.c_shr_mac = Vec::new();
        self.delta_x_clear = Vec::new();
        self.delta_y_clear = Vec::new();
    }
}

impl<S: Spdz2kShare> Gate<S> for MultiplyGate<S> {
    impl_gate_common!(MultiplyGate, S);

    fn do_read_offline_from_file(&mut self, party: &mut PartyWithFakeOffline<S>) {
        self.do_read_offline_base(party);
    }

    fn do_run_online(&mut self, party: &mut PartyWithFakeOffline<S>) {
        self.do_run_online_base(party);
    }
}
