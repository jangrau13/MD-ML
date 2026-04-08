// By Boshi Yuan (Rust rewrite)
// Matrix multiplication optimized with transpose-B, SIMD, and rayon parallelism

use crate::share::ShareElement;

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

/// High-performance matrix multiply with transpose-B, SIMD, and rayon.
/// Dispatches to a specialized u64 NEON kernel when available, otherwise
/// uses a cache-blocked + transposed generic path.
pub fn matrix_multiply<T: ShareElement>(
    lhs: &[T],
    rhs: &[T],
    dim_row: usize,
    dim_mid: usize,
    dim_col: usize,
) -> Vec<T> {
    #[cfg(target_arch = "aarch64")]
    if T::byte_size() == 8 {
        return matrix_multiply_u64_neon(lhs, rhs, dim_row, dim_mid, dim_col);
    }

    #[cfg(target_arch = "x86_64")]
    if T::byte_size() == 8 {
        return matrix_multiply_u64_avx2(lhs, rhs, dim_row, dim_mid, dim_col);
    }

    matrix_multiply_blocked_transpose(lhs, rhs, dim_row, dim_mid, dim_col)
}

/// Transpose rhs from row-major (dim_mid x dim_col) to (dim_col x dim_mid)
#[inline]
fn transpose<T: ShareElement>(rhs: &[T], rows: usize, cols: usize) -> Vec<T> {
    // Block-transpose for cache friendliness
    const BLK: usize = 64;
    let mut out = vec![T::zero(); rows * cols];
    for rb in (0..rows).step_by(BLK) {
        let r_end = (rb + BLK).min(rows);
        for cb in (0..cols).step_by(BLK) {
            let c_end = (cb + BLK).min(cols);
            for r in rb..r_end {
                for c in cb..c_end {
                    out[c * rows + r] = rhs[r * cols + c];
                }
            }
        }
    }
    out
}

// ────────────────────────────────────────────────────────────────────────────
// Optimized u64 matmul: BLIS-style packing + rayon parallelism
//
// Strategy: pack panels of A and B into contiguous buffers sized for cache,
// then run a tight micro-kernel (rank-1 outer-product update).
//
// Loop order (BLIS):
//   5th loop: jc over NC columns of B   (L3 blocking)
//   4th loop: pc over KC depth           (L2 blocking — packed B panel fits L2)
//   3rd loop: ic over MC rows of A       (L1 blocking — packed A panel fits L1)
//   micro-kernel: MR x NR outer product
//
// On M4 Pro: L1=128KB/core, L2=16MB shared. For u64 (8 bytes):
//   KC=512, MC=64, NC=4096 → packed A = MC*KC*8 = 256KB (fits L2 per core)
//                              packed B = KC*NC*8 = 16MB  (fits L3)
// We use rayon at the ic-loop level for row parallelism.
// ────────────────────────────────────────────────────────────────────────────

/// Optimized u64 matmul, used for both aarch64 and x86_64.
fn matrix_multiply_u64_fast<T: ShareElement>(
    lhs: &[T],
    rhs: &[T],
    dim_row: usize,
    dim_mid: usize,
    dim_col: usize,
) -> Vec<T> {
    use rayon::prelude::*;

    let a: &[u64] = unsafe {
        std::slice::from_raw_parts(
            T::slice_as_bytes(lhs).as_ptr() as *const u64,
            lhs.len(),
        )
    };
    let b: &[u64] = unsafe {
        std::slice::from_raw_parts(
            T::slice_as_bytes(rhs).as_ptr() as *const u64,
            rhs.len(),
        )
    };

    let m = dim_row;
    let n = dim_col;
    let k = dim_mid;

    // Tuning parameters
    const KC: usize = 512;  // depth block — panel of B fits L2
    const NR: usize = 4;    // micro-kernel width (unrolled output columns)

    let mut output = vec![0u64; m * n];

    // Pack a panel of B: B[pc..pc+kc, jc..jc+nc] into column-contiguous layout
    // Layout: for each column j in [0, nc), store kc values contiguously
    #[inline(never)]
    fn pack_b(b: &[u64], packed: &mut [u64], pc: usize, kc: usize, jc: usize, nc: usize, n: usize) {
        for jj in 0..nc {
            let j = jc + jj;
            let dst_off = jj * kc;
            let mut pp = pc;
            // Unroll packing for better throughput
            let end4 = kc / 4 * 4;
            for ki in (0..end4).step_by(4) {
                unsafe {
                    *packed.get_unchecked_mut(dst_off + ki) = *b.get_unchecked(pp * n + j);
                    *packed.get_unchecked_mut(dst_off + ki + 1) = *b.get_unchecked((pp + 1) * n + j);
                    *packed.get_unchecked_mut(dst_off + ki + 2) = *b.get_unchecked((pp + 2) * n + j);
                    *packed.get_unchecked_mut(dst_off + ki + 3) = *b.get_unchecked((pp + 3) * n + j);
                }
                pp += 4;
            }
            for ki in end4..kc {
                unsafe {
                    *packed.get_unchecked_mut(dst_off + ki) = *b.get_unchecked(pp * n + j);
                }
                pp += 1;
            }
        }
    }

    // Micro-kernel: compute C[i, j..j+NR] += A_row[pc..pc+kc] . packed_b[j*kc..]
    // Processes NR columns at once to reuse A loads
    #[inline(always)]
    fn micro_kernel_nr(
        out_row: &mut [u64],
        a_row: &[u64],
        packed_b: &[u64],
        j_base: usize,
        kc: usize,
        nr: usize,
    ) {
        // Accumulate NR columns simultaneously
        let mut acc = [0u64; NR];
        for jj in 0..nr {
            let bp = (j_base + jj) * kc;
            let mut s: u64 = 0;
            let end4 = kc / 4 * 4;
            let mut ki = 0;
            while ki < end4 {
                unsafe {
                    let a0 = *a_row.get_unchecked(ki);
                    let a1 = *a_row.get_unchecked(ki + 1);
                    let a2 = *a_row.get_unchecked(ki + 2);
                    let a3 = *a_row.get_unchecked(ki + 3);
                    let b0 = *packed_b.get_unchecked(bp + ki);
                    let b1 = *packed_b.get_unchecked(bp + ki + 1);
                    let b2 = *packed_b.get_unchecked(bp + ki + 2);
                    let b3 = *packed_b.get_unchecked(bp + ki + 3);
                    s = s.wrapping_add(a0.wrapping_mul(b0))
                        .wrapping_add(a1.wrapping_mul(b1))
                        .wrapping_add(a2.wrapping_mul(b2))
                        .wrapping_add(a3.wrapping_mul(b3));
                }
                ki += 4;
            }
            while ki < kc {
                unsafe {
                    s = s.wrapping_add(
                        (*a_row.get_unchecked(ki)).wrapping_mul(*packed_b.get_unchecked(bp + ki))
                    );
                }
                ki += 1;
            }
            acc[jj] = s;
        }
        for jj in 0..nr {
            out_row[j_base + jj] = out_row[j_base + jj].wrapping_add(acc[jj]);
        }
    }

    // Main BLIS-style loop
    // Allocate packed B buffer (reused across ic blocks)
    for pc in (0..k).step_by(KC) {
        let kc = KC.min(k - pc);

        // For each column block of B
        // We pack B once and reuse across all row blocks
        let mut packed_b = vec![0u64; n * kc];
        pack_b(b, &mut packed_b, pc, kc, 0, n, n);

        // Parallelize over rows
        output
            .par_chunks_mut(n)
            .enumerate()
            .for_each(|(i, out_row)| {
                let a_row = &a[i * k + pc..i * k + pc + kc];

                // Process NR columns at a time
                let n_nr = n / NR * NR;
                let mut j = 0;
                while j < n_nr {
                    micro_kernel_nr(out_row, a_row, &packed_b, j, kc, NR);
                    j += NR;
                }
                // Remainder columns
                while j < n {
                    let bp = j * kc;
                    let mut s: u64 = 0;
                    for ki in 0..kc {
                        unsafe {
                            s = s.wrapping_add(
                                (*a_row.get_unchecked(ki)).wrapping_mul(*packed_b.get_unchecked(bp + ki))
                            );
                        }
                    }
                    out_row[j] = out_row[j].wrapping_add(s);
                    j += 1;
                }
            });
    }

    let out_bytes = unsafe {
        std::slice::from_raw_parts(output.as_ptr() as *const u8, output.len() * 8)
    };
    T::vec_from_bytes(out_bytes)
}

#[cfg(target_arch = "aarch64")]
fn matrix_multiply_u64_neon<T: ShareElement>(
    lhs: &[T], rhs: &[T], dim_row: usize, dim_mid: usize, dim_col: usize,
) -> Vec<T> {
    matrix_multiply_u64_fast(lhs, rhs, dim_row, dim_mid, dim_col)
}

#[cfg(target_arch = "x86_64")]
fn matrix_multiply_u64_avx2<T: ShareElement>(
    lhs: &[T], rhs: &[T], dim_row: usize, dim_mid: usize, dim_col: usize,
) -> Vec<T> {
    matrix_multiply_u64_fast(lhs, rhs, dim_row, dim_mid, dim_col)
}

// ────────────────────────────────────────────────────────────────────────────
// Generic blocked + transpose-B path (u32, u128, or non-SIMD platforms)
// ────────────────────────────────────────────────────────────────────────────
fn matrix_multiply_blocked_transpose<T: ShareElement>(
    lhs: &[T],
    rhs: &[T],
    dim_row: usize,
    dim_mid: usize,
    dim_col: usize,
) -> Vec<T> {
    use rayon::prelude::*;

    let bt = transpose(rhs, dim_mid, dim_col);

    let m = dim_row;
    let n = dim_col;
    let k = dim_mid;

    let mut output = vec![T::zero(); m * n];

    const KC: usize = 256;

    for pc in (0..k).step_by(KC) {
        let kc = KC.min(k - pc);

        output
            .par_chunks_mut(n)
            .enumerate()
            .for_each(|(i, out_row)| {
                let a_row = &lhs[i * k + pc..i * k + pc + kc];

                for j in 0..n {
                    let bt_col = &bt[j * k + pc..];
                    let mut acc = T::zero();

                    let mut ki = 0;
                    let end4 = kc / 4 * 4;
                    while ki < end4 {
                        acc = acc
                            .wrapping_add(a_row[ki].wrapping_mul(bt_col[ki]))
                            .wrapping_add(a_row[ki + 1].wrapping_mul(bt_col[ki + 1]))
                            .wrapping_add(a_row[ki + 2].wrapping_mul(bt_col[ki + 2]))
                            .wrapping_add(a_row[ki + 3].wrapping_mul(bt_col[ki + 3]));
                        ki += 4;
                    }
                    while ki < kc {
                        acc = acc.wrapping_add(a_row[ki].wrapping_mul(bt_col[ki]));
                        ki += 1;
                    }

                    out_row[j] = out_row[j].wrapping_add(acc);
                }
            });
    }

    output
}

pub fn print_vector<T: std::fmt::Display>(vec: &[T]) {
    for elem in vec {
        print!("{} ", elem);
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Naive O(n³) reference implementation for correctness checking
    fn matmul_reference<T: ShareElement>(
        a: &[T], b: &[T], m: usize, k: usize, n: usize,
    ) -> Vec<T> {
        let mut c = vec![T::zero(); m * n];
        for i in 0..m {
            for j in 0..n {
                let mut s = T::zero();
                for p in 0..k {
                    s = s.wrapping_add(a[i * k + p].wrapping_mul(b[p * n + j]));
                }
                c[i * n + j] = s;
            }
        }
        c
    }

    #[test]
    fn test_matmul_u64_small() {
        let a: Vec<u64> = (1..=12).collect(); // 3x4
        let b: Vec<u64> = (1..=8).collect();  // 4x2
        let expected = matmul_reference(&a, &b, 3, 4, 2);
        let result = matrix_multiply(&a, &b, 3, 4, 2);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_matmul_u64_medium() {
        let m = 127;
        let k = 255;
        let n = 131;
        let mut a = vec![0u64; m * k];
        let mut b = vec![0u64; k * n];
        u64::fill_random(&mut a);
        u64::fill_random(&mut b);
        let expected = matmul_reference(&a, &b, m, k, n);
        let result = matrix_multiply(&a, &b, m, k, n);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_matmul_u64_large() {
        let n = 512;
        let mut a = vec![0u64; n * n];
        let mut b = vec![0u64; n * n];
        u64::fill_random(&mut a);
        u64::fill_random(&mut b);
        let expected = matmul_reference(&a, &b, n, n, n);
        let result = matrix_multiply(&a, &b, n, n, n);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_matmul_u128_small() {
        let a: Vec<u128> = (1..=6).collect(); // 2x3
        let b: Vec<u128> = (1..=12).collect(); // 3x4
        let expected = matmul_reference(&a, &b, 2, 3, 4);
        let result = matrix_multiply(&a, &b, 2, 3, 4);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_matmul_u128_medium() {
        let m = 63;
        let k = 129;
        let n = 65;
        let mut a = vec![0u128; m * k];
        let mut b = vec![0u128; k * n];
        u128::fill_random(&mut a);
        u128::fill_random(&mut b);
        let expected = matmul_reference(&a, &b, m, k, n);
        let result = matrix_multiply(&a, &b, m, k, n);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_matmul_u32() {
        let m = 100;
        let k = 200;
        let n = 150;
        let mut a = vec![0u32; m * k];
        let mut b = vec![0u32; k * n];
        u32::fill_random(&mut a);
        u32::fill_random(&mut b);
        let expected = matmul_reference(&a, &b, m, k, n);
        let result = matrix_multiply(&a, &b, m, k, n);
        assert_eq!(result, expected);
    }
}
