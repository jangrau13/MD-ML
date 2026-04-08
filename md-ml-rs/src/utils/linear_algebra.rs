// By Boshi Yuan (Rust rewrite)

use crate::share::ShareElement;
use nalgebra::DMatrix;

pub fn matrix_add<T: ShareElement>(x: &[T], y: &[T]) -> Vec<T> {
    x.iter().zip(y.iter()).map(|(&a, &b)| a.wrapping_add(b)).collect()
}

pub fn matrix_add_assign<T: ShareElement>(x: &mut [T], y: &[T]) {
    for (a, &b) in x.iter_mut().zip(y.iter()) {
        *a = a.wrapping_add(b);
    }
}

pub fn matrix_add_constant<T: ShareElement>(x: &[T], constant: T) -> Vec<T> {
    x.iter().map(|&v| v.wrapping_add(constant)).collect()
}

pub fn matrix_subtract<T: ShareElement>(x: &[T], y: &[T]) -> Vec<T> {
    x.iter().zip(y.iter()).map(|(&a, &b)| a.wrapping_sub(b)).collect()
}

pub fn matrix_subtract_assign<T: ShareElement>(x: &mut [T], y: &[T]) {
    for (a, &b) in x.iter_mut().zip(y.iter()) {
        *a = a.wrapping_sub(b);
    }
}

pub fn matrix_scalar<T: ShareElement>(x: &[T], scalar: T) -> Vec<T> {
    x.iter().map(|&v| scalar.wrapping_mul(v)).collect()
}

pub fn matrix_scalar_assign<T: ShareElement>(x: &mut [T], scalar: T) {
    for v in x.iter_mut() {
        *v = scalar.wrapping_mul(*v);
    }
}

pub fn matrix_elem_multiply<T: ShareElement>(x: &[T], y: &[T]) -> Vec<T> {
    x.iter().zip(y.iter()).map(|(&a, &b)| a.wrapping_mul(b)).collect()
}

/// Matrix multiplication using nalgebra for u64 elements.
/// For u128 elements, falls back to a naive implementation since nalgebra
/// doesn't support u128.
pub fn matrix_multiply<T: ShareElement>(
    lhs: &[T],
    rhs: &[T],
    dim_row: usize,
    dim_mid: usize,
    dim_col: usize,
) -> Vec<T> {
    // Use nalgebra for u64 (the common case)
    if T::byte_size() == 8 {
        return matrix_multiply_nalgebra_u64(lhs, rhs, dim_row, dim_mid, dim_col);
    }
    // Fallback to naive for other sizes (u32, u128)
    matrix_multiply_naive(lhs, rhs, dim_row, dim_mid, dim_col)
}

fn matrix_multiply_nalgebra_u64<T: ShareElement>(
    lhs: &[T],
    rhs: &[T],
    dim_row: usize,
    dim_mid: usize,
    dim_col: usize,
) -> Vec<T> {
    // Safety: We know T is u64 here (byte_size == 8), convert via raw bytes
    let lhs_u64: Vec<u64> = lhs.iter().map(|v| {
        let bytes = v.to_le_bytes_vec();
        u64::from_le_bytes(bytes.try_into().unwrap())
    }).collect();
    let rhs_u64: Vec<u64> = rhs.iter().map(|v| {
        let bytes = v.to_le_bytes_vec();
        u64::from_le_bytes(bytes.try_into().unwrap())
    }).collect();

    // nalgebra uses column-major, so we need to handle row-major data
    // Create matrices from row-major data by transposing
    let mat_lhs = DMatrix::from_row_slice(dim_row, dim_mid, &lhs_u64);
    let mat_rhs = DMatrix::from_row_slice(dim_mid, dim_col, &rhs_u64);

    let result = mat_lhs * mat_rhs;

    // Extract row-major data
    let mut output = Vec::with_capacity(dim_row * dim_col);
    for r in 0..dim_row {
        for c in 0..dim_col {
            let val = result[(r, c)];
            let bytes = val.to_le_bytes();
            output.push(T::from_le_bytes(&bytes));
        }
    }
    output
}

fn matrix_multiply_naive<T: ShareElement>(
    lhs: &[T],
    rhs: &[T],
    dim_row: usize,
    dim_mid: usize,
    dim_col: usize,
) -> Vec<T> {
    use rayon::prelude::*;

    // Block size tuned for L1 cache (~32KB). For u128 (16 bytes),
    // a 32x32 block = 16KB which fits comfortably.
    const BLOCK: usize = 64;

    let mut output = vec![T::zero(); dim_row * dim_col];

    // Parallelize over row blocks
    output
        .par_chunks_mut(dim_col)
        .enumerate()
        .for_each(|(i, out_row)| {
            for kb in (0..dim_mid).step_by(BLOCK) {
                let k_end = (kb + BLOCK).min(dim_mid);
                for jb in (0..dim_col).step_by(BLOCK) {
                    let j_end = (jb + BLOCK).min(dim_col);
                    for k in kb..k_end {
                        let a = lhs[i * dim_mid + k];
                        for j in jb..j_end {
                            out_row[j] = out_row[j].wrapping_add(a.wrapping_mul(rhs[k * dim_col + j]));
                        }
                    }
                }
            }
        });

    output
}

pub fn print_vector<T: std::fmt::Display>(vec: &[T]) {
    for elem in vec {
        print!("{} ", elem);
    }
    println!();
}
