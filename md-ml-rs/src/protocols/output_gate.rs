// By Boshi Yuan (Rust rewrite)

use crate::share::{ShareElement, Spdz2kShare};
use crate::protocols::gate::*;
use crate::protocols::PartyWithFakeOffline;
use crate::utils::linear_algebra::{matrix_add_assign, matrix_subtract};
use std::io::{Read, Write};
use std::thread;

pub struct OutputGate<S: Spdz2kShare> {
    pub data: GateData<S>,
    output_value: Vec<S::SemiShrType>,
}

impl<S: Spdz2kShare> OutputGate<S> {
    pub fn new(input_x: GateRef<S>) -> Self {
        let (dim_row, dim_col) = {
            let g = input_x.lock().unwrap();
            (g.dim_row(), g.dim_col())
        };
        let mut data = GateData::new_with_inputs(input_x, None);
        data.dim_row = dim_row;
        data.dim_col = dim_col;
        OutputGate {
            data,
            output_value: Vec::new(),
        }
    }

    pub fn get_clear(&self) -> Vec<S::ClearType> {
        self.output_value
            .iter()
            .map(|&v| {
                let bytes = v.to_le_bytes_vec();
                S::ClearType::from_le_bytes(&bytes)
            })
            .collect()
    }
}

impl<S: Spdz2kShare> Gate<S> for OutputGate<S> {
    impl_gate_common!(OutputGate, S);

    fn do_read_offline_from_file(&mut self, _party: &mut PartyWithFakeOffline<S>) {
        // Do nothing
    }

    fn do_run_online(&mut self, party: &mut PartyWithFakeOffline<S>) {
        let size = self.data.dim_row * self.data.dim_col;

        // Get lambda_shr and delta_clear from input_x
        let (input_lambda_shr, input_delta_clear) = {
            let ix = self.data.input_x.as_ref().unwrap().lock().unwrap();
            (ix.lambda_shr().clone(), ix.delta_clear().clone())
        };

        // Send lambda_shr and receive lambda_shr from other party concurrently
        let total_bytes = size * S::SemiShrType::byte_size();

        // Zero-copy serialization
        let send_bytes = S::SemiShrType::slice_as_bytes(&input_lambda_shr).to_vec();

        let other_id = 1 - party.my_id();
        let (send_stream, recv_stream) = party.party.split_streams(other_id);

        // Clone streams for threading
        let mut send_clone = send_stream.try_clone().expect("Failed to clone send stream");
        let mut recv_clone = recv_stream.try_clone().expect("Failed to clone recv stream");

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

        // Zero-copy deserialization
        let mut lambda_clear = S::SemiShrType::vec_from_bytes(&recv_buf);

        // Reconstruct lambda_x
        matrix_add_assign(&mut lambda_clear, &input_lambda_shr);

        // x = Delta_x - lambda_x
        self.output_value = matrix_subtract(&input_delta_clear, &lambda_clear);
    }
}
