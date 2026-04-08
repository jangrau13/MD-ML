"""
Fake offline phase: trusted dealer that generates Beaver triples and shares.

Mirrors the Rust FakeParty / FakeCircuit / FakeGate hierarchy.
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any

from .share import ShareConfig, _wrap128
from .linear_algebra import matrix_multiply, matrix_subtract


class FakeParty:
    """Trusted dealer that generates shares for N parties."""

    def __init__(self, share: ShareConfig, job_name: str, fake_offline_dir: str, n_parties: int = 2):
        self.share = share
        self.n_parties = n_parties
        self._dir = Path(fake_offline_dir)
        self._dir.mkdir(parents=True, exist_ok=True)

        sep = "-" if job_name else ""
        self._files = []
        for i in range(n_parties):
            path = self._dir / f"{job_name}{sep}party-{i}.txt"
            self._files.append(open(path, "wb"))

        # Generate MAC key: global_key = sum of key_shares
        self._key_shares = [int(share.rand_key()) for _ in range(n_parties)]
        self._global_key = sum(self._key_shares) & ((1 << share.ks_bits) - 1)

        # Write MAC key shares to files (widened to GlobalKeyType)
        for i in range(n_parties):
            self._files[i].write(
                self._key_shares[i].to_bytes(share.global_key_bytes, "little")
            )

    def close(self):
        for f in self._files:
            f.close()

    def generate_shares_vec(self, values) -> dict[str, list]:
        """
        Generate all parties' shares for a vector of ClearType values.
        Returns dict with 'value_shares' and 'mac_shares', each a list of N arrays.
        """
        s = self.share
        n = self.n_parties
        size = len(values)
        ks_bits = s.ks_bits
        mask = (1 << ks_bits) - 1
        k_bits = s.k_bits

        # Generate masks
        masks = s.rand_semi(size)

        # Build masked values and MACs
        if s.semi_dtype is not None:
            import numpy as np
            # numpy path (32-bit or 64-bit combined ring)
            values_wide = s.widen_clear_to_semi(values)

            # For Spdz2kShare32: mask is u64, value is u32->u64
            # masked_value = (mask << K) + value_wide  (wrapping)
            # But we need random mask in KeyShrType range, not SemiShrType
            key_masks = [int(s.rand_key()) for _ in range(size)]
            key_masks_arr = np.array(key_masks, dtype=s.semi_dtype)
            masked_values = (key_masks_arr << np.array(k_bits, dtype=s.semi_dtype)) + values_wide

            global_key_semi = np.array(self._global_key, dtype=s.semi_dtype)
            macs = masked_values * global_key_semi

            # Generate shares
            value_shares = [None] * n
            mac_shares = [None] * n
            for i in range(n - 1):
                value_shares[i] = s.rand_semi(size)
                mac_shares[i] = s.rand_semi(size)

            v_sum = np.zeros(size, dtype=s.semi_dtype)
            m_sum = np.zeros(size, dtype=s.semi_dtype)
            for i in range(n - 1):
                v_sum += value_shares[i]
                m_sum += mac_shares[i]
            value_shares[n - 1] = masked_values - v_sum
            mac_shares[n - 1] = macs - m_sum
        else:
            # Python int path (128-bit)
            key_masks = [int(s.rand_key()) for _ in range(size)]
            if hasattr(values, '__len__') and hasattr(values, '__getitem__'):
                vals_list = [int(v) for v in values]
            else:
                vals_list = list(values)

            masked_values = [
                _wrap128((km << k_bits) + v) for km, v in zip(key_masks, vals_list)
            ]
            macs = [_wrap128(mv * self._global_key) for mv in masked_values]

            value_shares = [None] * n
            mac_shares = [None] * n
            for i in range(n - 1):
                value_shares[i] = s.rand_semi(size)
                mac_shares[i] = s.rand_semi(size)

            if isinstance(value_shares[0], list):
                v_sum = [0] * size
                m_sum = [0] * size
                for i in range(n - 1):
                    for j in range(size):
                        v_sum[j] = _wrap128(v_sum[j] + value_shares[i][j])
                        m_sum[j] = _wrap128(m_sum[j] + mac_shares[i][j])
                value_shares[n - 1] = [_wrap128(mv - vs) for mv, vs in zip(masked_values, v_sum)]
                mac_shares[n - 1] = [_wrap128(mc - ms) for mc, ms in zip(macs, m_sum)]
            else:
                import numpy as np
                v_sum = np.zeros(size, dtype=value_shares[0].dtype)
                m_sum = np.zeros(size, dtype=value_shares[0].dtype)
                for i in range(n - 1):
                    v_sum += value_shares[i]
                    m_sum += mac_shares[i]
                value_shares[n - 1] = np.array(masked_values, dtype=v_sum.dtype) - v_sum
                mac_shares[n - 1] = np.array(macs, dtype=m_sum.dtype) - m_sum

        return {"value_shares": value_shares, "mac_shares": mac_shares}

    def write_shares_to_all(self, shares: list):
        """Write per-party shares (list of N arrays) to all party files."""
        for i in range(self.n_parties):
            self._files[i].write(self.share.semi_to_bytes(shares[i]))

    def write_clear_to_all(self, values):
        """Write clear values (widened to SemiShrType) to all party files."""
        widened = self.share.widen_clear_to_semi(values)
        data = self.share.semi_to_bytes(widened)
        for i in range(self.n_parties):
            self._files[i].write(data)

    def write_shares_to_party(self, values, party_id: int):
        """Write SemiShrType values to a specific party's file."""
        self._files[party_id].write(self.share.semi_to_bytes(values))


# ---------------------------------------------------------------------------
# Fake gates
# ---------------------------------------------------------------------------

class FakeGateData:
    def __init__(self):
        self.dim_row: int = 0
        self.dim_col: int = 0
        self.lambda_clear = None
        self.lambda_shr: list | None = None    # [N] arrays
        self.lambda_shr_mac: list | None = None
        self.input_x: FakeGate | None = None
        self.input_y: FakeGate | None = None
        self.done: bool = False


class FakeGate:
    def __init__(self):
        self.data = FakeGateData()

    def run_offline(self, party: FakeParty):
        if self.data.done:
            return
        if self.data.input_x is not None:
            self.data.input_x.run_offline(party)
        if self.data.input_y is not None:
            self.data.input_y.run_offline(party)
        if not self.data.done:
            self._do_run_offline(party)
            self.data.done = True

    def _do_run_offline(self, party: FakeParty):
        raise NotImplementedError


class FakeInputGate(FakeGate):
    def __init__(self, dim_row: int, dim_col: int, owner_id: int):
        super().__init__()
        self.data.dim_row = dim_row
        self.data.dim_col = dim_col
        self.owner_id = owner_id

    def _do_run_offline(self, party: FakeParty):
        s = party.share
        size = self.data.dim_row * self.data.dim_col
        self.data.lambda_clear = s.rand_clear(size)

        shares = party.generate_shares_vec(self.data.lambda_clear)
        self.data.lambda_shr = shares["value_shares"]
        self.data.lambda_shr_mac = shares["mac_shares"]

        # Write lambda_clear (widened) to owner only
        widened = s.widen_clear_to_semi(self.data.lambda_clear)
        party.write_shares_to_party(widened, self.owner_id)

        # Write shares to all parties
        party.write_shares_to_all(self.data.lambda_shr)
        party.write_shares_to_all(self.data.lambda_shr_mac)


class FakeOutputGate(FakeGate):
    def __init__(self, input_x: FakeGate):
        super().__init__()
        self.data.dim_row = input_x.data.dim_row
        self.data.dim_col = input_x.data.dim_col
        self.data.input_x = input_x

    def _do_run_offline(self, party: FakeParty):
        pass  # Nothing to do


class FakeMultiplyGate(FakeGate):
    def __init__(self, input_x: FakeGate, input_y: FakeGate):
        super().__init__()
        assert input_x.data.dim_col == input_y.data.dim_row, \
            "Incompatible dimensions for multiply gate"
        self.data.dim_row = input_x.data.dim_row
        self.data.dim_col = input_y.data.dim_col
        self.dim_mid = input_x.data.dim_col
        self.data.input_x = input_x
        self.data.input_y = input_y

    def _do_run_offline(self, party: FakeParty):
        s = party.share
        dim_row = self.data.dim_row
        dim_mid = self.dim_mid
        dim_col = self.data.dim_col
        size_lhs = dim_row * dim_mid
        size_rhs = dim_mid * dim_col
        size_out = dim_row * dim_col

        # Generate lambda_z
        self.data.lambda_clear = s.rand_clear(size_out)
        lz = party.generate_shares_vec(self.data.lambda_clear)
        self.data.lambda_shr = lz["value_shares"]
        self.data.lambda_shr_mac = lz["mac_shares"]

        # Generate Beaver triple: a, b, c = a * b
        a_clear = s.rand_clear(size_lhs)
        b_clear = s.rand_clear(size_rhs)
        c_clear = matrix_multiply(a_clear, b_clear, dim_row, dim_mid, dim_col)

        a_shares = party.generate_shares_vec(a_clear)
        b_shares = party.generate_shares_vec(b_clear)
        c_shares = party.generate_shares_vec(c_clear)

        # delta_x = a - lambda_x, delta_y = b - lambda_y
        ix_lambda = self.data.input_x.data.lambda_clear
        iy_lambda = self.data.input_y.data.lambda_clear
        delta_x_clear = matrix_subtract(a_clear, ix_lambda)
        delta_y_clear = matrix_subtract(b_clear, iy_lambda)

        # Write all data to files
        party.write_shares_to_all(a_shares["value_shares"])
        party.write_shares_to_all(a_shares["mac_shares"])
        party.write_shares_to_all(b_shares["value_shares"])
        party.write_shares_to_all(b_shares["mac_shares"])
        party.write_shares_to_all(c_shares["value_shares"])
        party.write_shares_to_all(c_shares["mac_shares"])
        party.write_shares_to_all(self.data.lambda_shr)
        party.write_shares_to_all(self.data.lambda_shr_mac)
        party.write_clear_to_all(delta_x_clear)
        party.write_clear_to_all(delta_y_clear)


# ---------------------------------------------------------------------------
# Fake circuit
# ---------------------------------------------------------------------------

class FakeCircuit:
    def __init__(self, share: ShareConfig):
        self.share = share
        self._gates: list[FakeGate] = []
        self._endpoints: list[FakeGate] = []

    def input(self, owner_id: int, dim_row: int, dim_col: int) -> FakeGate:
        g = FakeInputGate(dim_row, dim_col, owner_id)
        self._gates.append(g)
        return g

    def multiply(self, input_x: FakeGate, input_y: FakeGate) -> FakeGate:
        g = FakeMultiplyGate(input_x, input_y)
        self._gates.append(g)
        return g

    def output(self, input_x: FakeGate) -> FakeGate:
        g = FakeOutputGate(input_x)
        self._gates.append(g)
        return g

    def add_endpoint(self, gate: FakeGate):
        self._endpoints.append(gate)

    def run_offline(self, party: FakeParty):
        for gate in self._endpoints:
            gate.run_offline(party)
        party.close()
