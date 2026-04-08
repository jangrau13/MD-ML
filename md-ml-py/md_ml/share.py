"""
SPDZ-2k share types with wrapping modular arithmetic.

All arithmetic is performed modulo 2^N using numpy arrays of the appropriate
unsigned integer dtype. For the "combined" ring Z_{2^{K+S}} where K+S > 64
we fall back to Python int arrays (lists), since numpy has no native u128.
"""

from __future__ import annotations

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


def _rand_int128(size: int) -> list[int]:
    """Generate a list of random 128-bit unsigned integers."""
    buf = os.urandom(size * 16)
    return [int.from_bytes(buf[i * 16:(i + 1) * 16], "little") for i in range(size)]


# ---------------------------------------------------------------------------
# 128-bit wrapping helpers (Python int lists, mod 2^128)
# ---------------------------------------------------------------------------
_MOD128 = 1 << 128
_MASK128 = _MOD128 - 1


def _wrap128(x: int) -> int:
    return x & _MASK128


def _add128(a: list[int], b: list[int]) -> list[int]:
    return [_wrap128(x + y) for x, y in zip(a, b)]


def _sub128(a: list[int], b: list[int]) -> list[int]:
    return [_wrap128(x - y) for x, y in zip(a, b)]


def _mul128(a: list[int], b: list[int]) -> list[int]:
    return [_wrap128(x * y) for x, y in zip(a, b)]


def _scalar128(a: list[int], s: int) -> list[int]:
    return [_wrap128(x * s) for x, y in zip(a, [s] * len(a))]


def _to_bytes_128(values: list[int]) -> bytes:
    return b"".join(v.to_bytes(16, "little") for v in values)


def _from_bytes_128(buf: bytes, count: int) -> list[int]:
    return [int.from_bytes(buf[i * 16:(i + 1) * 16], "little") for i in range(count)]


# ---------------------------------------------------------------------------
# Share configuration dataclass
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class ShareConfig:
    """Configuration for a SPDZ-2k share scheme."""
    k_bits: int          # Value ring Z_{2^K}
    s_bits: int          # MAC key ring Z_{2^S}

    @property
    def ks_bits(self) -> int:
        return self.k_bits + self.s_bits

    # --- numpy dtypes for the value ring ---
    @property
    def clear_dtype(self) -> np.dtype | None:
        """dtype for ClearType (K bits). None if > 64."""
        return {32: np.dtype(np.uint32), 64: np.dtype(np.uint64)}.get(self.k_bits)

    @property
    def semi_dtype(self) -> np.dtype | None:
        """dtype for SemiShrType (K+S bits). None if > 64."""
        return {32: np.dtype(np.uint32), 64: np.dtype(np.uint64)}.get(self.ks_bits)

    @property
    def key_dtype(self) -> np.dtype | None:
        """dtype for KeyShrType (S bits). None if > 64."""
        return {32: np.dtype(np.uint32), 64: np.dtype(np.uint64)}.get(self.s_bits)

    @property
    def global_key_dtype(self) -> np.dtype | None:
        """dtype for GlobalKeyType (K+S bits). Same as semi_dtype."""
        return self.semi_dtype

    # --- Byte sizes ---
    @property
    def clear_bytes(self) -> int:
        return self.k_bits // 8

    @property
    def semi_bytes(self) -> int:
        return self.ks_bits // 8

    @property
    def key_bytes(self) -> int:
        return self.s_bits // 8

    @property
    def global_key_bytes(self) -> int:
        return self.ks_bits // 8

    # --- Operations ---
    def remove_upper_bits(self, values):
        """Z_{2^{K+S}} -> Z_{2^K} (mask off upper S bits)."""
        mask = (1 << self.k_bits) - 1
        if isinstance(values, np.ndarray):
            return values & np.array(mask, dtype=values.dtype)
        else:
            # list of Python ints (128-bit)
            return [v & mask for v in values]

    def widen_clear_to_semi(self, values):
        """Widen ClearType values to SemiShrType."""
        if self.semi_dtype is not None and isinstance(values, np.ndarray):
            return values.astype(self.semi_dtype)
        elif isinstance(values, np.ndarray):
            # numpy -> Python int list (for 128-bit)
            return [int(v) for v in values]
        else:
            return list(values)

    def widen_key_to_global(self, value):
        """Widen a single KeyShrType to GlobalKeyType."""
        return int(value)

    def widen_global_to_semi(self, value):
        """Widen GlobalKeyType to SemiShrType."""
        return int(value)

    def rand_clear(self, size: int):
        """Random ClearType array."""
        if self.clear_dtype is not None:
            return _rand_array(size, self.clear_dtype)
        return _rand_int128(size)

    def rand_semi(self, size: int):
        """Random SemiShrType array."""
        if self.semi_dtype is not None:
            return _rand_array(size, self.semi_dtype)
        return _rand_int128(size)

    def rand_key(self):
        """Random single KeyShrType value."""
        if self.key_dtype is not None:
            return _rand_array(1, self.key_dtype)[0]
        return _rand_int128(1)[0]

    def zeros_semi(self, size: int):
        """Zero-filled SemiShrType array."""
        if self.semi_dtype is not None:
            return np.zeros(size, dtype=self.semi_dtype)
        return [0] * size

    def semi_to_bytes(self, values) -> bytes:
        """Serialize SemiShrType values to little-endian bytes."""
        if isinstance(values, np.ndarray):
            return values.tobytes()
        return _to_bytes_128(values)

    def semi_from_bytes(self, buf: bytes, count: int):
        """Deserialize SemiShrType values from little-endian bytes."""
        if self.semi_dtype is not None:
            return np.frombuffer(buf, dtype=self.semi_dtype).copy()
        return _from_bytes_128(buf, count)

    def global_key_to_bytes(self, value) -> bytes:
        """Serialize a single GlobalKeyType value."""
        return int(value).to_bytes(self.global_key_bytes, "little")

    def global_key_from_bytes(self, buf: bytes):
        """Deserialize a single GlobalKeyType value."""
        return int.from_bytes(buf[:self.global_key_bytes], "little")

    # --- Wrapping arithmetic on SemiShrType ---
    def semi_add(self, a, b):
        if isinstance(a, np.ndarray):
            return a + b  # numpy uint wraps naturally
        return _add128(a, b)

    def semi_sub(self, a, b):
        if isinstance(a, np.ndarray):
            return a - b
        return _sub128(a, b)

    def semi_mul_elem(self, a, b):
        if isinstance(a, np.ndarray):
            return a * b
        return _mul128(a, b)

    def semi_scalar(self, a, s):
        """Multiply SemiShrType array by a scalar."""
        if isinstance(a, np.ndarray):
            return a * np.array(s, dtype=a.dtype)
        return [_wrap128(x * s) for x in a]

    def semi_add_assign(self, a, b):
        if isinstance(a, np.ndarray):
            a += b
            return a
        for i in range(len(a)):
            a[i] = _wrap128(a[i] + b[i])
        return a

    def semi_sub_assign(self, a, b):
        if isinstance(a, np.ndarray):
            a -= b
            return a
        for i in range(len(a)):
            a[i] = _wrap128(a[i] - b[i])
        return a


# --- Concrete share types ---

Spdz2kShare32 = ShareConfig(k_bits=32, s_bits=32)
Spdz2kShare64 = ShareConfig(k_bits=64, s_bits=64)
