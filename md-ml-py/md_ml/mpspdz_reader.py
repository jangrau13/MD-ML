"""
Reader for MP-SPDZ Persistence files (SPDZ-2k format).

Reads preprocessing material persisted by the bench_simple.mpc program
via sint.write_to_file(). The data is stored in:
  Persistence/Transactions-P<id>.data

Format (SPDZ-2k with k bits, s bits):
  Ring element size: ceil((k+s)/8) bytes, little-endian
  Share = (value[elem_bytes], mac[elem_bytes])

File layout (written by bench_simple.mpc for n×n matmul):
  Header: 8-byte length prefix + payload (type string + spec + mac key)
  Position 0     .. n²-1:     [a] shares (matrix Beaver triple)
  Position n²    .. 2n²-1:    [b] shares
  Position 2n²   .. 3n²-1:    [c] shares (c = matmul(a, b))
  Position 3n²   .. 5n²-1:    input mask shares (λ_A then λ_B)

MAC key file: Player-Data/{subdir}/Player-MAC-Keys-R-P<id>
"""

from __future__ import annotations

import math
import os
import struct
from pathlib import Path
from typing import NamedTuple

from .share import ShareConfig


class MpSpdzReader:
    """Reads MP-SPDZ Persistence files for a given SPDZ-2k configuration."""

    def __init__(self, share: ShareConfig, n_parties: int, party_id: int,
                 player_data_dir: str = "Player-Data",
                 n_elements: int = 4):
        self.share = share
        self.n_parties = n_parties
        self.party_id = party_id
        self.elem_bytes = math.ceil(share.ks_bits / 8)
        self.n_elements = n_elements  # n² for an n×n matrix

        type_short = f"Z{share.k_bits},{share.s_bits}"
        base = Path(player_data_dir)
        sub_dir = base / f"{n_parties}-{type_short}-{share.k_bits}"
        self.base_dir = sub_dir if sub_dir.exists() else base
        self.type_short = type_short

        persistence_dir = Path(os.environ.get("PERSISTENCE_DIR", str(base / "Persistence")))
        self.persistence_path = persistence_dir / f"Transactions-P{party_id}.data"

    def _read_elem(self, f) -> int:
        """Read one ring element (Z_{2^{k+s}}) from file."""
        buf = f.read(self.elem_bytes)
        if len(buf) < self.elem_bytes:
            raise EOFError(f"Expected {self.elem_bytes} bytes, got {len(buf)}")
        return int.from_bytes(buf, "little")

    def _skip_header(self, f):
        """Skip the MP-SPDZ file signature header."""
        length_buf = f.read(8)
        if len(length_buf) < 8:
            raise EOFError("Empty or corrupt file")
        payload_len = struct.unpack("<Q", length_buf)[0]
        f.read(payload_len)

    def _open_persistence(self):
        """Open the persistence file and skip its header."""
        f = open(self.persistence_path, "rb")
        self._skip_header(f)
        return f

    def _read_shares(self, f, count: int) -> tuple[list[int], list[int]]:
        """Read count shares, return (values, macs)."""
        vals, macs = [], []
        for _ in range(count):
            vals.append(self._read_elem(f))
            macs.append(self._read_elem(f))
        return vals, macs

    def _seek_to_position(self, f, position: int):
        """Skip past `position` shares (each = value + mac)."""
        skip_bytes = position * 2 * self.elem_bytes
        f.read(skip_bytes)

    def read_matrix_triple(self) -> tuple[list[int], list[int], list[int], list[int], list[int], list[int]]:
        """Read the matrix Beaver triple [a], [b], [c] where c = matmul(a, b).

        Returns: (a_val, a_mac, b_val, b_mac, c_val, c_mac)
        Each is a flat list of n² ints.
        """
        size = self.n_elements
        with self._open_persistence() as f:
            a_val, a_mac = self._read_shares(f, size)
            b_val, b_mac = self._read_shares(f, size)
            c_val, c_mac = self._read_shares(f, size)
        return a_val, a_mac, b_val, b_mac, c_val, c_mac

    def read_input_masks(self, count: int, mask_offset: int = 0) -> tuple[list[int], list[int]]:
        """Read input mask shares from the persistence file.

        Input masks start at position 3*n_elements (after a, b, c).
        mask_offset: number of masks to skip (e.g. n_elements for λ_B).

        Returns: (values, macs) — each a list of `count` ints.
        """
        with self._open_persistence() as f:
            # Skip past a, b, c sections + any prior masks
            self._seek_to_position(f, 3 * self.n_elements + mask_offset)
            return self._read_shares(f, count)

    def read_mac_key(self) -> int:
        """Read this party's MAC key share."""
        for name in [f"Player-MAC-Keys-R-P{self.party_id}",
                     f"Player-MAC-Keys-{self.type_short}-P{self.party_id}"]:
            path = self.base_dir / name
            if path.exists():
                with open(path, "rb") as f:
                    return self._read_elem(f)
        raise FileNotFoundError(
            f"MAC key file not found in {self.base_dir}. "
            f"Tried: Player-MAC-Keys-R-P{self.party_id}, "
            f"Player-MAC-Keys-{self.type_short}-P{self.party_id}"
        )

    def exists(self) -> bool:
        """Check if the persistence file exists."""
        return self.persistence_path.exists()
