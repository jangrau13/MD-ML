// By Shixuan Yang (Rust rewrite)

use crate::share::{ShareElement, Spdz2kShare};
use crate::protocols::gate::*;
use crate::protocols::PartyWithFakeOffline;
use crate::utils::linear_algebra::*;
use std::io::{Read, Write};
use std::thread;

pub struct GtzGate<S: Spdz2kShare> {
    pub data: GateData<S>,
    lambda_x_bin_shr: Vec<S::ClearType>,
    // Binary triples (currently faked as (0, 0, 0))
    a: bool,
    b: bool,
    c: bool,
}

impl<S: Spdz2kShare> GtzGate<S> {
    pub fn new(input_x: GateRef<S>) -> Self {
        let (dim_row, dim_col) = {
            let g = input_x.lock().unwrap();
            (g.dim_row(), g.dim_col())
        };
        let mut data = GateData::new_with_inputs(input_x, None);
        data.dim_row = dim_row;
        data.dim_col = dim_col;
        GtzGate {
            data,
            lambda_x_bin_shr: Vec::new(),
            a: false,
            b: false,
            c: false,
        }
    }

    fn bit_lt(
        &self,
        p_int: &[S::ClearType],
        s_int: &[S::ClearType],
        my_id: usize,
        party: &mut PartyWithFakeOffline<S>,
    ) -> Vec<bool> {
        let num_bits = S::ClearType::bit_count();
        let n = s_int.len();
        let mut b_: Vec<Vec<bool>> = vec![vec![false; num_bits]; n];
        let mut a_: Vec<Vec<bool>> = vec![vec![false; num_bits]; n];

        for i in 0..n {
            for j in 0..num_bits {
                let s_bit = get_bit(s_int[i], j);
                let p_bit = get_bit(p_int[i], j);
                if my_id == 0 {
                    b_[i][j] = !s_bit;
                } else {
                    b_[i][j] = s_bit;
                }
                a_[i][j] = p_bit;
            }
        }

        let mut s = self.carry_out_cin(&a_, &b_, true, my_id, party);
        if my_id == 0 {
            for v in s.iter_mut() {
                *v = !*v;
            }
        }
        s
    }

    fn carry_out_cin(
        &self,
        a_in: &[Vec<bool>],
        b_in: &[Vec<bool>],
        c_in: bool,
        my_id: usize,
        party: &mut PartyWithFakeOffline<S>,
    ) -> Vec<bool> {
        let num_bits = S::ClearType::bit_count();
        let n = b_in.len();
        let mut p: Vec<Vec<bool>> = vec![vec![false; num_bits]; n];
        let mut g: Vec<Vec<bool>> = vec![vec![false; num_bits]; n];

        for i in 0..n {
            for j in 0..num_bits {
                if my_id == 0 {
                    p[i][j] = a_in[i][j] ^ b_in[i][j];
                } else {
                    p[i][j] = b_in[i][j];
                }
                g[i][j] = a_in[i][j] & b_in[i][j];
            }
        }
        for i in 0..n {
            g[i][0] = g[i][0] ^ (c_in & p[i][0]);
        }

        self.carry_out_aux(&p, &g, num_bits, my_id, party)
    }

    fn carry_out_aux(
        &self,
        p: &[Vec<bool>],
        g: &[Vec<bool>],
        k: usize,
        my_id: usize,
        party: &mut PartyWithFakeOffline<S>,
    ) -> Vec<bool> {
        if k > 1 {
            let u_len = k / 2;
            let n = g.len();
            let num_triples = n * u_len * 2;

            let msg_bytes = (num_triples * 2 + 7) / 8;
            let mut sendmsg = vec![0u8; msg_bytes];
            let mut index_triple = 0;

            for i in 0..n {
                for j in 0..u_len {
                    let vec_loc = index_triple * 2 / 8;
                    let bit_loc = (index_triple * 2) % 8;
                    index_triple += 1;
                    sendmsg[vec_loc] |= ((p[i][2 * j] ^ self.a) as u8) << bit_loc;
                    sendmsg[vec_loc] |= ((p[i][2 * j + 1] ^ self.b) as u8) << (bit_loc + 1);

                    let vec_loc = index_triple * 2 / 8;
                    let bit_loc = (index_triple * 2) % 8;
                    index_triple += 1;
                    sendmsg[vec_loc] |= ((g[i][2 * j] ^ self.a) as u8) << bit_loc;
                    sendmsg[vec_loc] |= ((p[i][2 * j + 1] ^ self.b) as u8) << (bit_loc + 1);
                }
            }

            // Send MAC messages (placeholder, same as C++ code)
            let send_mac_msg = vec![S::ClearType::zero(); num_triples * 2];

            // Exchange
            let other_id = 1 - my_id;
            let (send_stream, recv_stream) = party.party.split_streams(other_id);
            let mut send_clone = send_stream.try_clone().unwrap();
            let mut recv_clone = recv_stream.try_clone().unwrap();

            let sendmsg_clone = sendmsg.clone();
            let send_mac_bytes: Vec<u8> = send_mac_msg.iter().flat_map(|v| v.to_le_bytes_vec()).collect();
            let clear_byte_size = S::ClearType::byte_size();
            let mac_total_bytes = num_triples * 2 * clear_byte_size;

            let t1 = thread::spawn(move || {
                send_clone.write_all(&sendmsg_clone).unwrap();
                send_clone.write_all(&send_mac_bytes).unwrap();
            });

            let t2 = thread::spawn(move || -> (Vec<u8>, Vec<u8>) {
                let mut rcvmsg = vec![0u8; msg_bytes];
                recv_clone.read_exact(&mut rcvmsg).unwrap();
                let mut rcv_mac = vec![0u8; mac_total_bytes];
                recv_clone.read_exact(&mut rcv_mac).unwrap();
                (rcvmsg, rcv_mac)
            });

            t1.join().unwrap();
            let (rcvmsg, _rcv_mac) = t2.join().unwrap();

            // Compute u_p, u_g
            let num_bits_total = std::cmp::max(k, S::ClearType::bit_count());
            let mut u_p: Vec<Vec<bool>> = vec![vec![false; num_bits_total]; n];
            let mut u_g: Vec<Vec<bool>> = vec![vec![false; num_bits_total]; n];

            index_triple = 0;
            for i in 0..n {
                for j in 0..u_len {
                    let vec_loc = index_triple * 2 / 8;
                    let bit_loc = (index_triple * 2) % 8;
                    index_triple += 1;

                    let alpha = get_byte_bit(&sendmsg, vec_loc, bit_loc) ^ get_byte_bit(&rcvmsg, vec_loc, bit_loc);
                    let beta = get_byte_bit(&sendmsg, vec_loc, bit_loc + 1) ^ get_byte_bit(&rcvmsg, vec_loc, bit_loc + 1);
                    let x_ = p[i][2 * j];
                    let y_ = p[i][2 * j + 1];
                    let mut z = self.c ^ (alpha & y_) ^ (beta & x_);
                    if my_id == 0 {
                        z ^= alpha & beta;
                    }
                    u_p[i][j] = z;

                    let vec_loc = index_triple * 2 / 8;
                    let bit_loc = (index_triple * 2) % 8;
                    index_triple += 1;

                    let alpha = get_byte_bit(&sendmsg, vec_loc, bit_loc) ^ get_byte_bit(&rcvmsg, vec_loc, bit_loc);
                    let beta = get_byte_bit(&sendmsg, vec_loc, bit_loc + 1) ^ get_byte_bit(&rcvmsg, vec_loc, bit_loc + 1);
                    let x_ = g[i][2 * j];
                    let y_ = p[i][2 * j + 1];
                    let mut z = self.c ^ (alpha & y_) ^ (beta & x_);
                    if my_id == 0 {
                        z ^= alpha & beta;
                    }
                    u_g[i][j] = g[i][2 * j + 1] ^ z;
                }
            }

            let mut actual_u_len = u_len;
            if k % 2 == 1 {
                for i in 0..n {
                    u_p[i][actual_u_len] = p[i][k - 1];
                    u_g[i][actual_u_len] = g[i][k - 1];
                }
                actual_u_len += 1;
            }

            self.carry_out_aux(&u_p, &u_g, actual_u_len, my_id, party)
        } else {
            // k <= 1
            g.iter().map(|gi| gi[0]).collect()
        }
    }
}

fn get_bit<T: ShareElement>(val: T, bit_idx: usize) -> bool {
    let shifted = val >> bit_idx;
    let one = T::one();
    (shifted & one) != T::zero()
}

fn get_byte_bit(buf: &[u8], byte_idx: usize, bit_idx: usize) -> bool {
    (buf[byte_idx] >> bit_idx) & 1 == 1
}

impl<S: Spdz2kShare> Gate<S> for GtzGate<S> {
    impl_gate_common!(GtzGate, S);

    fn do_read_offline_from_file(&mut self, party: &mut PartyWithFakeOffline<S>) {
        let size = self.data.dim_row * self.data.dim_col;
        self.data.lambda_shr = party.read_shares(size);
        self.data.lambda_shr_mac = party.read_shares(size);
        self.lambda_x_bin_shr = party.read_clear(size);
    }

    fn do_run_online(&mut self, party: &mut PartyWithFakeOffline<S>) {
        let my_id = party.my_id();

        let delta_x_semi = {
            let ix = self.data.input_x.as_ref().unwrap().lock().unwrap();
            ix.delta_clear().clone()
        };

        // Convert SemiShrType -> ClearType
        let delta_x: Vec<S::ClearType> = delta_x_semi
            .iter()
            .map(|v| {
                let bytes = v.to_le_bytes_vec();
                S::ClearType::from_le_bytes(&bytes)
            })
            .collect();

        let mut ret = self.bit_lt(&delta_x, &self.lambda_x_bin_shr, my_id, party);
        if my_id == 0 {
            for v in ret.iter_mut() {
                *v = !*v;
            }
        }

        // Pack bits into bytes and exchange
        let msg_bytes = (ret.len() + 7) / 8;
        let mut sendmsg = vec![0u8; msg_bytes];
        for j in 0..ret.len() {
            let vec_loc = j / 8;
            let bit_loc = j % 8;
            sendmsg[vec_loc] |= (ret[j] as u8) << bit_loc;
        }

        let other_id = 1 - my_id;
        let (send_stream, recv_stream) = party.party.split_streams(other_id);
        let mut send_clone = send_stream.try_clone().unwrap();
        let mut recv_clone = recv_stream.try_clone().unwrap();

        let sendmsg_clone = sendmsg.clone();
        let t1 = thread::spawn(move || {
            send_clone.write_all(&sendmsg_clone).unwrap();
        });
        let t2 = thread::spawn(move || -> Vec<u8> {
            let mut buf = vec![0u8; msg_bytes];
            recv_clone.read_exact(&mut buf).unwrap();
            buf
        });
        t1.join().unwrap();
        let rcvmsg = t2.join().unwrap();

        let mut rcv = vec![false; ret.len()];
        for j in 0..rcv.len() {
            rcv[j] = get_byte_bit(&rcvmsg, j / 8, j % 8);
        }

        let mut z_shr: Vec<S::SemiShrType> = vec![S::SemiShrType::zero(); ret.len()];
        if my_id == 0 {
            for j in 0..ret.len() {
                if ret[j] ^ rcv[j] {
                    z_shr[j] = S::SemiShrType::one();
                }
            }
        }

        self.data.delta_clear = matrix_add(&self.data.lambda_shr, &z_shr);

        // Exchange delta_clear
        let total_bytes = z_shr.len() * S::SemiShrType::byte_size();
        let send_bytes = S::SemiShrType::slice_as_bytes(&self.data.delta_clear).to_vec();

        let (send_stream, recv_stream) = party.party.split_streams(other_id);
        let mut send_clone = send_stream.try_clone().unwrap();
        let mut recv_clone = recv_stream.try_clone().unwrap();

        let t3 = thread::spawn(move || {
            const CHUNK: usize = 1 << 20;
            for chunk in send_bytes.chunks(CHUNK) {
                send_clone.write_all(chunk).unwrap();
            }
        });
        let t4 = thread::spawn(move || -> Vec<u8> {
            let mut buf = vec![0u8; total_bytes];
            recv_clone.read_exact(&mut buf).unwrap();
            buf
        });
        t3.join().unwrap();
        let delta_rcv_buf = t4.join().unwrap();

        let delta_rcv = S::SemiShrType::vec_from_bytes(&delta_rcv_buf);

        matrix_add_assign(&mut self.data.delta_clear, &delta_rcv);
    }
}
