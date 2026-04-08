// By Boshi Yuan (Rust rewrite)

use crate::share::{ShareElement, Spdz2kShare};
use crate::protocols::gate::*;
use crate::protocols::PartyWithFakeOffline;
use crate::utils::linear_algebra::*;
use std::io::{Read, Write};
use std::thread;

pub struct ElemMultiplyGate<S: Spdz2kShare> {
    pub data: GateData<S>,
    a_shr: Vec<S::SemiShrType>,
    a_shr_mac: Vec<S::SemiShrType>,
    b_shr: Vec<S::SemiShrType>,
    b_shr_mac: Vec<S::SemiShrType>,
    c_shr: Vec<S::SemiShrType>,
    c_shr_mac: Vec<S::SemiShrType>,
    delta_x_clear: Vec<S::SemiShrType>,
    delta_y_clear: Vec<S::SemiShrType>,
}

impl<S: Spdz2kShare> ElemMultiplyGate<S> {
    pub fn new(input_x: GateRef<S>, input_y: GateRef<S>) -> Self {
        let (dim_row, dim_col) = {
            let gx = input_x.lock().unwrap();
            let gy = input_y.lock().unwrap();
            assert!(
                gx.dim_row() == gy.dim_row() && gx.dim_col() == gy.dim_col(),
                "The inputs of element-wise multiplication gate should have compatible dimensions"
            );
            (gx.dim_row(), gx.dim_col())
        };
        let mut data = GateData::new_with_inputs(input_x, Some(input_y));
        data.dim_row = dim_row;
        data.dim_col = dim_col;
        ElemMultiplyGate {
            data,
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
}

impl<S: Spdz2kShare> Gate<S> for ElemMultiplyGate<S> {
    impl_gate_common!(ElemMultiplyGate, S);

    fn do_read_offline_from_file(&mut self, party: &mut PartyWithFakeOffline<S>) {
        let size = self.data.dim_row * self.data.dim_col;
        self.a_shr = party.read_shares(size);
        self.a_shr_mac = party.read_shares(size);
        self.b_shr = party.read_shares(size);
        self.b_shr_mac = party.read_shares(size);
        self.c_shr = party.read_shares(size);
        self.c_shr_mac = party.read_shares(size);
        self.data.lambda_shr = party.read_shares(size);
        self.data.lambda_shr_mac = party.read_shares(size);
        self.delta_x_clear = party.read_shares(size);
        self.delta_y_clear = party.read_shares(size);
    }

    fn do_run_online(&mut self, party: &mut PartyWithFakeOffline<S>) {
        let my_id = party.my_id();
        let global_key_shr = party.global_key_shr();

        let (delta_x, delta_y) = {
            let ix = self.data.input_x.as_ref().unwrap().lock().unwrap();
            let iy = self.data.input_y.as_ref().unwrap().lock().unwrap();
            (ix.delta_clear().clone(), iy.delta_clear().clone())
        };

        let temp_x = matrix_add(&delta_x, &self.delta_x_clear);
        let temp_y = matrix_add(&delta_y, &self.delta_y_clear);
        let temp_xy = matrix_elem_multiply(&temp_x, &temp_y);

        let mut delta_z_shr = matrix_add(&self.c_shr, &self.data.lambda_shr);
        matrix_subtract_assign(&mut delta_z_shr, &matrix_elem_multiply(&self.a_shr, &temp_y));
        matrix_subtract_assign(&mut delta_z_shr, &matrix_elem_multiply(&temp_x, &self.b_shr));
        if my_id == 0 {
            matrix_add_assign(&mut delta_z_shr, &temp_xy);
        }

        // Widen global_key_shr
        let key_wide = {
            let bytes = global_key_shr.to_le_bytes_vec();
            let mut extended = vec![0u8; S::SemiShrType::byte_size()];
            extended[..bytes.len()].copy_from_slice(&bytes);
            S::SemiShrType::from_le_bytes(&extended)
        };

        let mut delta_z_mac = matrix_scalar(&temp_xy, key_wide);
        matrix_add_assign(&mut delta_z_mac, &self.c_shr_mac);
        matrix_add_assign(&mut delta_z_mac, &self.data.lambda_shr_mac);
        matrix_subtract_assign(&mut delta_z_mac, &matrix_elem_multiply(&self.a_shr_mac, &temp_y));
        matrix_subtract_assign(&mut delta_z_mac, &matrix_elem_multiply(&temp_x, &self.b_shr_mac));

        // Exchange concurrently
        let byte_size = S::SemiShrType::byte_size();
        let size = self.data.dim_row * self.data.dim_col;
        let total_bytes = size * byte_size;

        let mut send_buf = Vec::with_capacity(total_bytes);
        for val in &delta_z_shr {
            send_buf.extend_from_slice(&val.to_le_bytes_vec());
        }

        let other_id = 1 - my_id;
        let (send_stream, recv_stream) = party.party.split_streams(other_id);
        let mut send_clone = send_stream.try_clone().expect("Failed to clone");
        let mut recv_clone = recv_stream.try_clone().expect("Failed to clone");

        let t1 = thread::spawn(move || {
            send_clone.write_all(&send_buf).expect("Failed to send");
        });
        let t2 = thread::spawn(move || -> Vec<u8> {
            let mut buf = vec![0u8; total_bytes];
            recv_clone.read_exact(&mut buf).expect("Failed to receive");
            buf
        });

        t1.join().expect("Send thread panicked");
        let recv_buf = t2.join().expect("Recv thread panicked");

        self.data.delta_clear = recv_buf
            .chunks_exact(byte_size)
            .map(S::SemiShrType::from_le_bytes)
            .collect();

        matrix_add_assign(&mut self.data.delta_clear, &delta_z_shr);
        S::remove_upper_bits_inplace(&mut self.data.delta_clear);

        // Free
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
