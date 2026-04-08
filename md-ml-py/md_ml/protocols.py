"""
Online phase: gates, circuit, and party with fake offline data.

Mirrors the Rust protocols module.
"""

from __future__ import annotations

import time
from pathlib import Path

from .share import ShareConfig
from .networking import Party
from .linear_algebra import (
    matrix_multiply, matrix_add, matrix_subtract,
    matrix_add_assign, matrix_subtract_assign, matrix_scalar,
)


# ---------------------------------------------------------------------------
# Party with fake offline data reader
# ---------------------------------------------------------------------------

class PartyWithFakeOffline:
    """Reads preprocessed data from a file and provides networking."""

    def __init__(
        self,
        share: ShareConfig,
        my_id: int,
        num_parties: int,
        port: int,
        job_name: str,
        fake_offline_dir: str,
    ):
        self.share = share
        self.party = Party(my_id, num_parties, port)
        self._my_id = my_id

        sep = "-" if job_name else ""
        path = Path(fake_offline_dir) / f"{job_name}{sep}party-{my_id}.txt"
        self._file = open(path, "rb")

        # Read MAC key
        buf = self._file.read(share.global_key_bytes)
        self._global_key_shr = share.global_key_from_bytes(buf)

    @property
    def my_id(self) -> int:
        return self._my_id

    @property
    def global_key_shr(self):
        return self._global_key_shr

    @property
    def bytes_sent(self) -> int:
        return self.party.bytes_sent

    def read_shares(self, count: int):
        """Read count SemiShrType values from the offline file."""
        nbytes = count * self.share.semi_bytes
        buf = self._file.read(nbytes)
        assert len(buf) == nbytes, f"EOF: expected {nbytes} bytes, got {len(buf)}"
        return self.share.semi_from_bytes(buf, count)


# ---------------------------------------------------------------------------
# Gate data and base class
# ---------------------------------------------------------------------------

class GateData:
    def __init__(self):
        self.dim_row: int = 0
        self.dim_col: int = 0
        self.lambda_shr = None
        self.lambda_shr_mac = None
        self.delta_clear = None
        self.input_x: Gate | None = None
        self.input_y: Gate | None = None
        self.read_offline: bool = False
        self.evaluated_online: bool = False


class Gate:
    def __init__(self):
        self.data = GateData()

    def read_offline_from_file(self, party: PartyWithFakeOffline):
        if self.data.read_offline:
            return
        if self.data.input_x is not None:
            self.data.input_x.read_offline_from_file(party)
        if self.data.input_y is not None:
            self.data.input_y.read_offline_from_file(party)
        if not self.data.read_offline:
            self._do_read_offline(party)
            self.data.read_offline = True

    def run_online(self, party: PartyWithFakeOffline):
        if self.data.evaluated_online:
            return
        if self.data.input_x is not None:
            self.data.input_x.run_online(party)
        if self.data.input_y is not None:
            self.data.input_y.run_online(party)
        if not self.data.evaluated_online:
            self._do_run_online(party)
            self.data.evaluated_online = True

    def _do_read_offline(self, party: PartyWithFakeOffline):
        raise NotImplementedError

    def _do_run_online(self, party: PartyWithFakeOffline):
        raise NotImplementedError


# ---------------------------------------------------------------------------
# Input gate
# ---------------------------------------------------------------------------

class InputGate(Gate):
    def __init__(self, dim_row: int, dim_col: int, owner_id: int):
        super().__init__()
        self.data.dim_row = dim_row
        self.data.dim_col = dim_col
        self.owner_id = owner_id
        self._lambda_clear = None
        self._input_value = None

    def set_input(self, values, my_id: int, share: ShareConfig):
        assert my_id == self.owner_id, "Not the owner of this input gate"
        assert len(values) == self.data.dim_row * self.data.dim_col
        self._input_value = share.widen_clear_to_semi(values)

    def _do_read_offline(self, party: PartyWithFakeOffline):
        size = self.data.dim_row * self.data.dim_col
        if party.my_id == self.owner_id:
            self._lambda_clear = party.read_shares(size)
        self.data.lambda_shr = party.read_shares(size)
        self.data.lambda_shr_mac = party.read_shares(size)

    def _do_run_online(self, party: PartyWithFakeOffline):
        s = party.share
        size = self.data.dim_row * self.data.dim_col
        other_id = 1 - party.my_id

        if party.my_id == self.owner_id:
            self.data.delta_clear = matrix_add(self._input_value, self._lambda_clear)
            party.party.send_bytes_to_other(s.semi_to_bytes(self.data.delta_clear))
        else:
            nbytes = size * s.semi_bytes
            buf = party.party.recv_bytes_from_other(nbytes)
            self.data.delta_clear = s.semi_from_bytes(buf, size)


# ---------------------------------------------------------------------------
# Output gate
# ---------------------------------------------------------------------------

class OutputGate(Gate):
    def __init__(self, input_x: Gate):
        super().__init__()
        self.data.dim_row = input_x.data.dim_row
        self.data.dim_col = input_x.data.dim_col
        self.data.input_x = input_x
        self._output_value = None

    def _do_read_offline(self, party: PartyWithFakeOffline):
        pass

    def _do_run_online(self, party: PartyWithFakeOffline):
        s = party.share
        ix = self.data.input_x
        size = self.data.dim_row * self.data.dim_col
        other_id = 1 - party.my_id

        input_lambda_shr = ix.data.lambda_shr
        input_delta_clear = ix.data.delta_clear

        # Exchange lambda_shr
        send_data = s.semi_to_bytes(input_lambda_shr)
        nbytes = size * s.semi_bytes
        recv_data = party.party.send_recv_concurrent(other_id, send_data, nbytes)

        lambda_clear = s.semi_from_bytes(recv_data, size)
        lambda_clear = matrix_add_assign(lambda_clear, input_lambda_shr)

        # x = Delta_x - lambda_x
        self._output_value = matrix_subtract(input_delta_clear, lambda_clear)

    def get_clear(self, share: ShareConfig):
        """Get the output as ClearType values."""
        return share.remove_upper_bits(self._output_value)


# ---------------------------------------------------------------------------
# Multiply gate
# ---------------------------------------------------------------------------

class MultiplyGate(Gate):
    def __init__(self, input_x: Gate, input_y: Gate):
        super().__init__()
        assert input_x.data.dim_col == input_y.data.dim_row
        self.data.dim_row = input_x.data.dim_row
        self.data.dim_col = input_y.data.dim_col
        self.dim_mid = input_x.data.dim_col
        self.data.input_x = input_x
        self.data.input_y = input_y

        self.a_shr = None
        self.a_shr_mac = None
        self.b_shr = None
        self.b_shr_mac = None
        self.c_shr = None
        self.c_shr_mac = None
        self.delta_x_clear = None
        self.delta_y_clear = None

    def _do_read_offline(self, party: PartyWithFakeOffline):
        size_lhs = self.data.dim_row * self.dim_mid
        size_rhs = self.dim_mid * self.data.dim_col
        size_out = self.data.dim_row * self.data.dim_col

        self.a_shr = party.read_shares(size_lhs)
        self.a_shr_mac = party.read_shares(size_lhs)
        self.b_shr = party.read_shares(size_rhs)
        self.b_shr_mac = party.read_shares(size_rhs)
        self.c_shr = party.read_shares(size_out)
        self.c_shr_mac = party.read_shares(size_out)
        self.data.lambda_shr = party.read_shares(size_out)
        self.data.lambda_shr_mac = party.read_shares(size_out)
        self.delta_x_clear = party.read_shares(size_lhs)
        self.delta_y_clear = party.read_shares(size_rhs)

    def _do_run_online(self, party: PartyWithFakeOffline):
        s = party.share
        dim_row = self.data.dim_row
        dim_mid = self.dim_mid
        dim_col = self.data.dim_col

        delta_x = self.data.input_x.data.delta_clear
        delta_y = self.data.input_y.data.delta_clear

        # temp_x = Delta_x + delta_x
        temp_x = matrix_add(delta_x, self.delta_x_clear)
        # temp_y = Delta_y + delta_y
        temp_y = matrix_add(delta_y, self.delta_y_clear)

        # temp_xy = temp_x * temp_y  (matmul 1/5)
        t = time.time()
        temp_xy = matrix_multiply(temp_x, temp_y, dim_row, dim_mid, dim_col)
        print(f"  [multiply] matmul 1/5 (temp_x * temp_y) {(time.time()-t)*1000:.0f} ms", flush=True)

        # [Delta_z] = [c] + [lambda_z]
        delta_z_shr = matrix_add(self.c_shr, self.data.lambda_shr)

        # [Delta_z] -= [a] * temp_y  (matmul 2/5)
        t = time.time()
        delta_z_shr = matrix_subtract_assign(
            delta_z_shr,
            matrix_multiply(self.a_shr, temp_y, dim_row, dim_mid, dim_col),
        )
        print(f"  [multiply] matmul 2/5 (a * temp_y) {(time.time()-t)*1000:.0f} ms", flush=True)

        # [Delta_z] -= temp_x * [b]  (matmul 3/5)
        t = time.time()
        delta_z_shr = matrix_subtract_assign(
            delta_z_shr,
            matrix_multiply(temp_x, self.b_shr, dim_row, dim_mid, dim_col),
        )
        print(f"  [multiply] matmul 3/5 (temp_x * b) {(time.time()-t)*1000:.0f} ms", flush=True)

        if party.my_id == 0:
            delta_z_shr = matrix_add_assign(delta_z_shr, temp_xy)

        # Compute Delta_z_mac
        global_key_wide = s.widen_global_to_semi(party.global_key_shr)
        delta_z_mac = matrix_scalar(temp_xy, global_key_wide)
        delta_z_mac = matrix_add_assign(delta_z_mac, self.c_shr_mac)
        delta_z_mac = matrix_add_assign(delta_z_mac, self.data.lambda_shr_mac)

        # matmul 4/5
        t = time.time()
        delta_z_mac = matrix_subtract_assign(
            delta_z_mac,
            matrix_multiply(self.a_shr_mac, temp_y, dim_row, dim_mid, dim_col),
        )
        print(f"  [multiply] matmul 4/5 (a_mac * temp_y) {(time.time()-t)*1000:.0f} ms", flush=True)

        # matmul 5/5
        t = time.time()
        delta_z_mac = matrix_subtract_assign(
            delta_z_mac,
            matrix_multiply(temp_x, self.b_shr_mac, dim_row, dim_mid, dim_col),
        )
        print(f"  [multiply] matmul 5/5 (temp_x * b_mac) {(time.time()-t)*1000:.0f} ms", flush=True)

        # Exchange Delta_z_shr
        other_id = 1 - party.my_id
        send_data = s.semi_to_bytes(delta_z_shr)
        nbytes = dim_row * dim_col * s.semi_bytes
        t = time.time()
        recv_data = party.party.send_recv_concurrent(other_id, send_data, nbytes)
        print(f"  [multiply] network exchange {(time.time()-t)*1000:.0f} ms", flush=True)

        self.data.delta_clear = s.semi_from_bytes(recv_data, dim_row * dim_col)
        self.data.delta_clear = matrix_add_assign(self.data.delta_clear, delta_z_shr)

        # Remove upper bits
        self.data.delta_clear = s.remove_upper_bits(self.data.delta_clear)

        # Free preprocessing data
        self.a_shr = None
        self.a_shr_mac = None
        self.b_shr = None
        self.b_shr_mac = None
        self.c_shr = None
        self.c_shr_mac = None
        self.delta_x_clear = None
        self.delta_y_clear = None


# ---------------------------------------------------------------------------
# Circuit
# ---------------------------------------------------------------------------

class Circuit:
    def __init__(self, share: ShareConfig):
        self.share = share
        self._gates: list[Gate] = []
        self._endpoints: list[Gate] = []
        self._elapsed_ms: float = 0

    def input(self, owner_id: int, dim_row: int, dim_col: int) -> InputGate:
        g = InputGate(dim_row, dim_col, owner_id)
        self._gates.append(g)
        return g

    def multiply(self, input_x: Gate, input_y: Gate) -> Gate:
        g = MultiplyGate(input_x, input_y)
        self._gates.append(g)
        return g

    def output(self, input_x: Gate) -> OutputGate:
        g = OutputGate(input_x)
        self._gates.append(g)
        return g

    def add_endpoint(self, gate: Gate):
        self._endpoints.append(gate)

    def read_offline_from_file(self, party: PartyWithFakeOffline):
        t = time.time()
        for gate in self._endpoints:
            gate.read_offline_from_file(party)
        print(f"[offline] Total offline reading: {(time.time()-t)*1000:.0f} ms")

    def run_online(self, party: PartyWithFakeOffline):
        for gate in self._endpoints:
            gate.run_online(party)

    def run_online_with_benchmark(self, party: PartyWithFakeOffline):
        t = time.time()
        self.run_online(party)
        self._elapsed_ms = (time.time() - t) * 1000

    def print_stats(self, party: PartyWithFakeOffline):
        print(f"Spent {self._elapsed_ms:.0f} ms")
        print(f"Sent {party.bytes_sent} bytes")
