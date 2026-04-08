"""
Matrix operations for SPDZ-2k protocol.

Matrices are stored as flat 1-D arrays (row-major), matching the Rust/C++
implementations.

For unsigned integer matmul we use a float64-BLAS trick: split each integer
into small chunks that fit in float64's 53-bit mantissa, multiply via BLAS,
then reassemble with wrapping arithmetic. This gives us hardware-accelerated
(AMX/AVX) performance for integer matmul.
"""

from __future__ import annotations

import numpy as np


# ---------------------------------------------------------------------------
# Float64-BLAS integer matmul
# ---------------------------------------------------------------------------

def _matmul_u64_via_f64(a: np.ndarray, b: np.ndarray) -> np.ndarray:
    """
    Exact u64 matrix multiply using float64 BLAS.

    Split each 64-bit value into 4 x 16-bit chunks. Each BLAS matmul
    accumulates at most K products of (2^16-1)^2 ≈ 2^32, so the max
    partial sum is K * 2^32. For K ≤ 2^21 (~2M) this fits in 53 bits.

    We need 4x4 = 16 BLAS calls, but each runs at full BLAS speed.
    The result is reconstructed mod 2^64.
    """
    # Split into 16-bit chunks: val = c0 + c1*2^16 + c2*2^32 + c3*2^48
    a0 = (a & 0xFFFF).astype(np.float64)
    a1 = ((a >> 16) & 0xFFFF).astype(np.float64)
    a2 = ((a >> 32) & 0xFFFF).astype(np.float64)
    a3 = ((a >> 48) & 0xFFFF).astype(np.float64)

    b0 = (b & 0xFFFF).astype(np.float64)
    b1 = ((b >> 16) & 0xFFFF).astype(np.float64)
    b2 = ((b >> 32) & 0xFFFF).astype(np.float64)
    b3 = ((b >> 48) & 0xFFFF).astype(np.float64)

    # C = A @ B mod 2^64
    # = sum over (i,j) pairs where shift = 16*(i+j):
    #   (ai @ bj) << shift,  but only keep bits 0..63
    #
    # Shift 0  (contributes bits 0-63):  a0@b0
    # Shift 16 (contributes bits 16-63): a0@b1 + a1@b0
    # Shift 32 (contributes bits 32-63): a0@b2 + a1@b1 + a2@b0
    # Shift 48 (contributes bits 48-63): a0@b3 + a1@b2 + a2@b1 + a3@b0
    # Shift ≥64: all overflow, contributes 0

    # For each group, we do the BLAS matmuls, convert back to u64, shift, and add.
    # To keep precision we process each BLAS result through _f64_to_u64 which
    # handles the float->int conversion carefully.

    # Shift 0: just a0 @ b0
    s0 = _f64_matmul_to_u64(a0, b0)

    # Shift 16: (a0@b1 + a1@b0). We can add the float results before converting
    # since max value = 2 * K * (2^16)^2 = 2K * 2^32, fits in 53 bits for K < 2^20
    s16 = _f64_matmul_to_u64(a0, b1) + _f64_matmul_to_u64(a1, b0)

    # Shift 32
    s32 = (_f64_matmul_to_u64(a0, b2) +
           _f64_matmul_to_u64(a1, b1) +
           _f64_matmul_to_u64(a2, b0))

    # Shift 48
    s48 = (_f64_matmul_to_u64(a0, b3) +
           _f64_matmul_to_u64(a1, b2) +
           _f64_matmul_to_u64(a2, b1) +
           _f64_matmul_to_u64(a3, b0))

    return s0 + (s16 << np.uint64(16)) + (s32 << np.uint64(32)) + (s48 << np.uint64(48))


def _f64_matmul_to_u64(a: np.ndarray, b: np.ndarray) -> np.ndarray:
    """BLAS matmul then convert float64 result to uint64."""
    c = a @ b
    # np.around handles the ±0.5 rounding from float accumulation
    return np.around(c).astype(np.uint64)


def _matmul_u128_via_f64(a_list: list[int], b_list: list[int],
                          m: int, k: int, n: int) -> list[int]:
    """
    Exact u128 matrix multiply using float64 BLAS.

    Split each 128-bit value into 8 x 16-bit chunks, do BLAS matmuls for
    each pair whose combined shift < 128, then reassemble with Python ints.

    Total: 36 BLAS matmuls (8+7+6+5+4+3+2+1).
    Each partial result has max value K * (2^16-1)^2 < K * 2^32.
    For K ≤ 2^21 this fits in float64's 53-bit mantissa.
    """
    MASK128 = (1 << 128) - 1

    # Extract 8 x 16-bit chunks as float64 matrices
    a_chunks = []
    b_chunks = []
    for ci in range(8):
        shift = 16 * ci
        ac = np.empty((m, k), dtype=np.float64)
        bc = np.empty((k, n), dtype=np.float64)
        for idx in range(m * k):
            ac[idx // k, idx % k] = float((a_list[idx] >> shift) & 0xFFFF)
        for idx in range(k * n):
            bc[idx // n, idx % n] = float((b_list[idx] >> shift) & 0xFFFF)
        a_chunks.append(ac)
        b_chunks.append(bc)

    # Compute all needed BLAS matmuls, grouped by combined shift
    # partials[s] = list of (m x n) u64 arrays for shift group s (in units of 16 bits)
    partials = [[] for _ in range(8)]  # shift groups 0..7 (shift = 16*group)
    for i in range(8):
        for j in range(8 - i):
            p = np.around(a_chunks[i] @ b_chunks[j]).astype(np.int64)
            partials[i + j].append(p)

    # Reassemble per-element with Python ints
    out = [0] * (m * n)
    # Flatten all partials
    flat_partials = []
    for group in range(8):
        flat_group = [p.ravel() for p in partials[group]]
        flat_partials.append(flat_group)

    for idx in range(m * n):
        v = 0
        for group in range(8):
            s = 0
            for p in flat_partials[group]:
                s += int(p[idx])
            v += s << (16 * group)
        out[idx] = v & MASK128
    return out


# ---------------------------------------------------------------------------
# Matrix multiply (public API)
# ---------------------------------------------------------------------------

def matrix_multiply(lhs, rhs, dim_row: int, dim_mid: int, dim_col: int,
                    *, mask: int | None = None):
    """
    Compute C = A @ B where A is (dim_row x dim_mid) and B is (dim_mid x dim_col).
    All stored as flat row-major arrays. Uses float64 BLAS for acceleration.

    For Python int lists, `mask` controls the wrapping modulus (2^bits - 1).
    If None, defaults to 2^128 - 1 for backwards compat.
    """
    if isinstance(lhs, np.ndarray):
        a = lhs.reshape(dim_row, dim_mid)
        b = rhs.reshape(dim_mid, dim_col)
        if lhs.dtype == np.uint64:
            return _matmul_u64_via_f64(a, b).ravel()
        elif lhs.dtype == np.uint32:
            return _matmul_u32_via_f64(a, b).ravel()
        else:
            return (a @ b).ravel()
    else:
        m = mask if mask is not None else ((1 << 128) - 1)
        return _matmul_pyint(lhs, rhs, dim_row, dim_mid, dim_col, m)


def _matmul_pyint(a: list[int], b: list[int],
                  m: int, k: int, n: int, mask: int) -> list[int]:
    """Pure-Python integer matmul mod mask+1. For small rings or 128-bit."""
    out = [0] * (m * n)
    for i in range(m):
        for j in range(n):
            s = 0
            for p in range(k):
                s += a[i * k + p] * b[p * n + j]
            out[i * n + j] = s & mask
    return out


def _matmul_u32_via_f64(a: np.ndarray, b: np.ndarray) -> np.ndarray:
    """Exact u32 matmul via float64 BLAS. 2 chunks of 16 bits → 4 BLAS calls."""
    a0 = (a & 0xFFFF).astype(np.float64)
    a1 = ((a >> 16) & 0xFFFF).astype(np.float64)
    b0 = (b & 0xFFFF).astype(np.float64)
    b1 = ((b >> 16) & 0xFFFF).astype(np.float64)

    # Go through u64 to avoid overflow issues in u32
    s0 = np.around(a0 @ b0).astype(np.uint64).astype(np.uint32)
    s16 = (np.around(a0 @ b1).astype(np.uint64).astype(np.uint32) +
           np.around(a1 @ b0).astype(np.uint64).astype(np.uint32))
    return s0 + (s16 << np.uint32(16))


# ---------------------------------------------------------------------------
# Element-wise operations
# ---------------------------------------------------------------------------

def matrix_add(x, y, *, mask: int | None = None):
    if isinstance(x, np.ndarray):
        return x + y
    m = mask if mask is not None else ((1 << 128) - 1)
    return [((a + b) & m) for a, b in zip(x, y)]


def matrix_subtract(x, y, *, mask: int | None = None):
    if isinstance(x, np.ndarray):
        return x - y
    m = mask if mask is not None else ((1 << 128) - 1)
    return [((a - b) & m) for a, b in zip(x, y)]


def matrix_add_assign(x, y, *, mask: int | None = None):
    if isinstance(x, np.ndarray):
        x += y
        return x
    m = mask if mask is not None else ((1 << 128) - 1)
    for i in range(len(x)):
        x[i] = (x[i] + y[i]) & m
    return x


def matrix_subtract_assign(x, y, *, mask: int | None = None):
    if isinstance(x, np.ndarray):
        x -= y
        return x
    m = mask if mask is not None else ((1 << 128) - 1)
    for i in range(len(x)):
        x[i] = (x[i] - y[i]) & m
    return x


def matrix_scalar(x, scalar, *, mask: int | None = None):
    if isinstance(x, np.ndarray):
        return x * np.array(scalar, dtype=x.dtype)
    m = mask if mask is not None else ((1 << 128) - 1)
    return [((v * scalar) & m) for v in x]
