"""
SPDZ-2k share types with wrapping modular arithmetic.

All arithmetic is performed modulo 2^N using numpy arrays of the appropriate
unsigned integer dtype. For rings that don't map to a numpy uint (e.g. k=5,
k+s=10, or k+s=128) we fall back to Python int arrays (lists).
"""

from __future__ import annotations

import math
import os
import struct
from dataclasses import dataclass
from typing import Protocol

import numpy as np


# ---------------------------------------------------------------------------
# Helper: wrapping arithmetic on numpy arrays (naturally wraps for uint dtypes)
# ---------------------------------------------------------------------------

def _rand_array(size: int, dtype: np.dtype) -> np.ndarray:
    """Generate a random array of the given unsigned integer dtype."""
    nbytes = size * dtype.itemsize
    buf = os.urandom(nbytes)
    return np.frombuffer(buf, dtype=dtype).copy()


# ---------------------------------------------------------------------------
# Generic Python-int wrapping helpers (mod 2^bits)
# ---------------------------------------------------------------------------

def _rand_int_list(size: int, bits: int) -> list[int]:
    """Generate a list of random integers mod 2^bits."""
    mask = (1 << bits) - 1
    byte_count = math.ceil(bits / 8)
    buf = os.urandom(size * byte_count)
    return [int.from_bytes(buf[i * byte_count:(i + 1) * byte_count], "little") & mask
            for i in range(size)]


def _wrap(x: int, mask: int) -> int:
    return x & mask


def _add_list(a: list[int], b: list[int], mask: int) -> list[int]:
    return [(x + y) & mask for x, y in zip(a, b)]


def _sub_list(a: list[int], b: list[int], mask: int) -> list[int]:
    return [(x - y) & mask for x, y in zip(a, b)]


def _mul_list(a: list[int], b: list[int], mask: int) -> list[int]:
    return [(x * y) & mask for x, y in zip(a, b)]


def _to_bytes_generic(values: list[int], byte_width: int) -> bytes:
    return b"".join(v.to_bytes(byte_width, "little") for v in values)


def _from_bytes_generic(buf: bytes, count: int, byte_width: int) -> list[int]:
    return [int.from_bytes(buf[i * byte_width:(i + 1) * byte_width], "little")
            for i in range(count)]


# ---------------------------------------------------------------------------
# Share configuration dataclass
# ---------------------------------------------------------------------------

def _dtype_for_bits(bits: int) -> np.dtype | None:
    """Return a numpy unsigned dtype that fits exactly `bits`, or None."""
    return {8: np.dtype(np.uint8), 16: np.dtype(np.uint16),
            32: np.dtype(np.uint32), 64: np.dtype(np.uint64)}.get(bits)


@dataclass(frozen=True)
class ShareConfig:
    """Configuration for a SPDZ-2k share scheme."""
    k_bits: int          # Value ring Z_{2^K}
    s_bits: int          # MAC key ring Z_{2^S}

    @property
    def ks_bits(self) -> int:
        return self.k_bits + self.s_bits

    @property
    def clear_mask(self) -> int:
        return (1 << self.k_bits) - 1

    @property
    def semi_mask(self) -> int:
        return (1 << self.ks_bits) - 1

    # --- numpy dtypes (None when no exact-fit dtype exists) ---
    @property
    def clear_dtype(self) -> np.dtype | None:
        return _dtype_for_bits(self.k_bits)

    @property
    def semi_dtype(self) -> np.dtype | None:
        return _dtype_for_bits(self.ks_bits)

    @property
    def key_dtype(self) -> np.dtype | None:
        return _dtype_for_bits(self.s_bits)

    @property
    def global_key_dtype(self) -> np.dtype | None:
        return self.semi_dtype

    # --- Byte sizes (ceiling for non-byte-aligned k) ---
    @property
    def clear_bytes(self) -> int:
        return math.ceil(self.k_bits / 8)

    @property
    def semi_bytes(self) -> int:
        return math.ceil(self.ks_bits / 8)

    @property
    def key_bytes(self) -> int:
        return math.ceil(self.s_bits / 8)

    @property
    def global_key_bytes(self) -> int:
        return math.ceil(self.ks_bits / 8)

    # --- Operations ---
    def remove_upper_bits(self, values):
        """Z_{2^{K+S}} -> Z_{2^K} (mask off upper S bits)."""
        mask = self.clear_mask
        if isinstance(values, np.ndarray):
            return values & np.array(mask, dtype=values.dtype)
        else:
            return [v & mask for v in values]

    def widen_clear_to_semi(self, values):
        """Widen ClearType values to SemiShrType."""
        if self.semi_dtype is not None and isinstance(values, np.ndarray):
            return values.astype(self.semi_dtype)
        elif isinstance(values, np.ndarray):
            return [int(v) for v in values]
        else:
            return list(values)

    def widen_key_to_global(self, value):
        return int(value)

    def widen_global_to_semi(self, value):
        return int(value)

    def rand_clear(self, size: int):
        """Random ClearType array (mod 2^k)."""
        if self.clear_dtype is not None:
            return _rand_array(size, self.clear_dtype)
        mask = self.clear_mask
        return [v & mask for v in _rand_int_list(size, self.k_bits)]

    def rand_semi(self, size: int):
        """Random SemiShrType array (mod 2^{k+s})."""
        if self.semi_dtype is not None:
            return _rand_array(size, self.semi_dtype)
        mask = self.semi_mask
        return [v & mask for v in _rand_int_list(size, self.ks_bits)]

    def rand_key(self):
        """Random single KeyShrType value."""
        if self.key_dtype is not None:
            return _rand_array(1, self.key_dtype)[0]
        mask = (1 << self.s_bits) - 1
        return _rand_int_list(1, self.s_bits)[0] & mask

    def zeros_semi(self, size: int):
        """Zero-filled SemiShrType array."""
        if self.semi_dtype is not None:
            return np.zeros(size, dtype=self.semi_dtype)
        return [0] * size

    def semi_to_bytes(self, values) -> bytes:
        """Serialize SemiShrType values to little-endian bytes."""
        if isinstance(values, np.ndarray):
            return values.tobytes()
        return _to_bytes_generic(values, self.semi_bytes)

    def semi_from_bytes(self, buf: bytes, count: int):
        """Deserialize SemiShrType values from little-endian bytes."""
        if self.semi_dtype is not None:
            return np.frombuffer(buf, dtype=self.semi_dtype).copy()
        return _from_bytes_generic(buf, count, self.semi_bytes)

    def global_key_to_bytes(self, value) -> bytes:
        return int(value).to_bytes(self.global_key_bytes, "little")

    def global_key_from_bytes(self, buf: bytes):
        return int.from_bytes(buf[:self.global_key_bytes], "little")

    # --- Wrapping arithmetic on SemiShrType ---
    def semi_add(self, a, b):
        if isinstance(a, np.ndarray):
            return a + b
        return _add_list(a, b, self.semi_mask)

    def semi_sub(self, a, b):
        if isinstance(a, np.ndarray):
            return a - b
        return _sub_list(a, b, self.semi_mask)

    def semi_mul_elem(self, a, b):
        if isinstance(a, np.ndarray):
            return a * b
        return _mul_list(a, b, self.semi_mask)

    def semi_scalar(self, a, s):
        """Multiply SemiShrType array by a scalar."""
        if isinstance(a, np.ndarray):
            return a * np.array(s, dtype=a.dtype)
        mask = self.semi_mask
        return [(x * s) & mask for x in a]

    def semi_add_assign(self, a, b):
        if isinstance(a, np.ndarray):
            a += b
            return a
        mask = self.semi_mask
        for i in range(len(a)):
            a[i] = (a[i] + b[i]) & mask
        return a

    def semi_sub_assign(self, a, b):
        if isinstance(a, np.ndarray):
            a -= b
            return a
        mask = self.semi_mask
        for i in range(len(a)):
            a[i] = (a[i] - b[i]) & mask
        return a


# --- Concrete share types ---

def make_share_config(k: int, s: int | None = None) -> ShareConfig:
    """Create a ShareConfig with k value bits and s security bits.
    If s is not given, defaults to s=k (standard SPDZ-2k)."""
    if s is None:
        s = k
    return ShareConfig(k_bits=k, s_bits=s)

Spdz2kShare32 = ShareConfig(k_bits=32, s_bits=32)
Spdz2kShare64 = ShareConfig(k_bits=64, s_bits=64)
