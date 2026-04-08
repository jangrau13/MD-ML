#!/usr/bin/env python3
"""
Data Party container — step-by-step interactive mode.

Each data party owns a subset of columns of the input matrices A and B.
It generates random float values, encodes them as fixed-point, collects
λ shares from the computation parties, reconstructs λ, computes Δ = input + λ,
and exposes Δ for the coordinator to assemble.

Columns are assigned round-robin: data party i owns columns i, i+4, i+8, ...

API:
  GET  /api/state     → full party state (steps, log, phase)
  POST /api/configure → { dim, d, k, n_data_parties, comp_party_urls: [...] }
  POST /api/step      → advance one step
  GET  /api/delta      → { ready, delta_col_a, delta_col_b, float_col_a, float_col_b, columns }
"""

import os
import sys
import json
import math
import random
import time
import threading
import urllib.request
import urllib.error
from http.server import HTTPServer, BaseHTTPRequestHandler

sys.path.insert(0, "/app")

from md_ml.dashboard import PartyState, _make_handler


def float_to_fixed(x: float, d: int, k: int) -> str:
    mask = (1 << k) - 1
    mod = 1 << k
    scaled = round(x * (2 ** d))
    bi = scaled if scaled >= 0 else (mod + scaled)
    return str(bi & mask)


def add_mod_k(a: str, b: str, k: int) -> str:
    mask = (1 << k) - 1
    return str((int(a) + int(b)) & mask)


class StepBuilder:
    def __init__(self):
        self._steps = []
        self._idx = 0
    def add(self, name, phase="online", description=""):
        self._steps.append({"name": name, "phase": phase, "description": description})
        return self
    def register(self, state):
        state.set_steps(self._steps)
    def wait(self, state):
        state.wait_for_step(self._idx)
        self._idx += 1


def columns_for_party(dp_id: int, n_data_parties: int, dim: int) -> list:
    if dp_id >= dim:
        return []
    return list(range(dp_id, dim, n_data_parties))


def main():
    dp_id = int(os.environ.get("DATA_PARTY_ID", "0"))
    n_data_parties = int(os.environ.get("N_DATA_PARTIES", "4"))
    dash_port = int(os.environ.get("DASH_PORT", "8080"))
    comp_party_urls_str = os.environ.get("COMP_PARTY_URLS", "http://party0:8080,http://party1:8080")
    comp_party_urls = [u.strip() for u in comp_party_urls_str.split(",")]

    state = PartyState(party_id=dp_id, role="data_party")

    # ── Extended handler with /api/delta ──
    delta_store = {"ready": False, "data": None}
    delta_lock = threading.Lock()

    def make_extended_handler(st):
        BaseHandler = _make_handler(st)
        class ExtHandler(BaseHandler):
            def do_GET(self):
                if self.path == "/api/delta":
                    with delta_lock:
                        if delta_store["data"] is not None:
                            self._json(delta_store["data"])
                        else:
                            self._json({"ready": False})
                else:
                    super().do_GET()
        return ExtHandler

    handler = make_extended_handler(state)
    server = HTTPServer(("0.0.0.0", dash_port), handler)
    t = threading.Thread(target=server.serve_forever, daemon=True)
    t.start()
    print(f"Data Party {dp_id} API on http://0.0.0.0:{dash_port}", flush=True)

    while True:
        # ── Wait for config (blocks until POST /api/configure) ──
        # Note: delta store is NOT cleared here — it stays available
        # until the next configure resets everything via set_config()
        config = state.wait_for_config()

        # Reset delta store now that a new computation is starting
        with delta_lock:
            delta_store["ready"] = False
            delta_store["data"] = None
        dim = config["dim"]
        d_bits = config.get("d", 20)
        k = config.get("k", 64)

        cols = columns_for_party(dp_id, n_data_parties, dim)
        n_elements = dim * len(cols)  # rows × owned columns

        if len(cols) == 0:
            state.log(f"Data Party {dp_id}: no columns to own for {dim}×{dim} matrix (dp_id >= dim)")
            state.update(phase="done", status="No columns — idle")
            continue

        col_str = ", ".join(str(c) for c in cols)
        state.log(f"Data Party {dp_id}: owns column(s) {col_str} of {dim}×{dim} matrices", {
            "columns": cols,
            "elements_per_matrix": n_elements,
            "d": d_bits,
            "k": k,
        })

        # ── Steps ──
        sb = StepBuilder()
        sb.add(f"Generate float columns {col_str} for A and B", "preprocessing",
               f"Sample {n_elements} random entries in [-10, 10]")
        sb.add(f"Fixed-point encode columns {col_str}", "preprocessing",
               f"x ↦ round(x · 2^{d_bits}) mod 2^{k}")
        sb.add(f"Poll computation parties for [λ]^0, [λ]^1", "online",
               "Collect λ shares for owned columns from both computation parties")
        sb.add(f"Reconstruct λ for columns {col_str}", "online",
               "λ = [λ]^0 + [λ]^1 mod 2^k")
        sb.add(f"Compute Δ = input + λ for columns {col_str}", "online",
               "Mask input so computation parties never see plaintext")
        sb.add(f"Expose Δ via /api/delta", "online",
               "Coordinator collects Δ from all data parties to assemble full matrices")
        sb.register(state)
        state.update(status="Ready — click Start")

        # ══ Step 0: Generate float columns ══
        sb.wait(state)
        state.log(f"═══ Generating Float Data ═══", level="crypto")
        random.seed()

        # Compute input range so matmul result fits in Z_{2^k} without overflow.
        # For n×n matmul: each output = sum of n products, scaled by 2^d.
        # max_val = sqrt(2^(k-1) / (n * 2^d))
        import math as _math
        max_val = _math.sqrt(2 ** (k - 1) / (dim * (2 ** d_bits)))
        # Round down to nearest 0.5 for clean values, minimum 0.5
        max_val = max(0.5, _math.floor(max_val * 2) / 2)

        def nonzero_random():
            while True:
                v = round(random.uniform(-max_val, max_val) * 100) / 100
                if v != 0.0:
                    return v

        state.log(f"Input range: [-{max_val}, {max_val}] (k={k}, d={d_bits}, n={dim})")
        float_col_a = [nonzero_random() for _ in range(n_elements)]
        float_col_b = [nonzero_random() for _ in range(n_elements)]
        state.log(f"Generated {n_elements} float values per matrix for columns {col_str}", {
            f"A columns {col_str}": float_col_a,
            f"B columns {col_str}": float_col_b,
        }, level="crypto")

        # Expose floats immediately so the dashboard can show A, B
        with delta_lock:
            delta_store["data"] = {
                "ready": False,
                "data_party_id": dp_id,
                "columns": cols,
                "dim": dim,
                "float_col_a": float_col_a,
                "float_col_b": float_col_b,
            }

        # ══ Step 1: Fixed-point encode ══
        sb.wait(state)
        state.log(f"═══ Fixed-Point Encoding ═══", level="crypto")
        fixed_col_a = [float_to_fixed(x, d_bits, k) for x in float_col_a]
        fixed_col_b = [float_to_fixed(x, d_bits, k) for x in float_col_b]
        state.log(f"Encoded {n_elements} values: x ↦ round(x · 2^{d_bits}) mod 2^{k}", {
            f"A_fixed columns {col_str}": fixed_col_a,
            f"B_fixed columns {col_str}": fixed_col_b,
        }, level="crypto")

        # ══ Step 2: Poll computation parties for λ shares ══
        sb.wait(state)
        state.log(f"═══ Collecting λ Shares ═══", level="crypto")
        state.log(f"Polling {len(comp_party_urls)} computation parties...")

        lambda_a_p0 = None
        lambda_b_p0 = None
        lambda_a_p1 = None
        lambda_b_p1 = None

        # Poll until both computation parties have exposed their λ
        while lambda_a_p0 is None or lambda_a_p1 is None:
            time.sleep(0.5)
            for ci, url in enumerate(comp_party_urls):
                try:
                    resp = urllib.request.urlopen(f"{url}/api/lambdas", timeout=5)
                    data = json.loads(resp.read().decode())
                    if data.get("ready"):
                        shr_a = data["lambda_a_share"]
                        shr_b = data["lambda_b_share"]
                        my_a = []
                        my_b = []
                        for row in range(dim):
                            for c in cols:
                                my_a.append(shr_a[row * dim + c])
                                my_b.append(shr_b[row * dim + c])
                        if ci == 0:
                            lambda_a_p0 = my_a
                            lambda_b_p0 = my_b
                        else:
                            lambda_a_p1 = my_a
                            lambda_b_p1 = my_b
                except Exception:
                    pass

        state.log(f"Received λ shares from both computation parties", {
            f"[λ_A]^0 (columns {col_str})": lambda_a_p0,
            f"[λ_A]^1 (columns {col_str})": lambda_a_p1,
            f"[λ_B]^0 (columns {col_str})": lambda_b_p0,
            f"[λ_B]^1 (columns {col_str})": lambda_b_p1,
        }, level="crypto")

        # ══ Step 3: Reconstruct λ ══
        sb.wait(state)
        state.log(f"═══ Reconstructing λ ═══", level="crypto")
        mask = (1 << k) - 1
        lambda_col_a = [str((int(a) + int(b)) & mask) for a, b in zip(lambda_a_p0, lambda_a_p1)]
        lambda_col_b = [str((int(a) + int(b)) & mask) for a, b in zip(lambda_b_p0, lambda_b_p1)]
        state.log(f"λ = [λ]^0 + [λ]^1 mod 2^{k} for columns {col_str}", {
            f"λ_A columns {col_str}": lambda_col_a,
            f"λ_B columns {col_str}": lambda_col_b,
        }, level="crypto")

        # ══ Step 4: Compute Δ ══
        sb.wait(state)
        state.log(f"═══ Computing Δ = input + λ ═══", level="crypto")
        delta_col_a = [add_mod_k(a, l, k) for a, l in zip(fixed_col_a, lambda_col_a)]
        delta_col_b = [add_mod_k(b, l, k) for b, l in zip(fixed_col_b, lambda_col_b)]
        state.log(f"Δ = input_fixed + λ (mod 2^{k}) for columns {col_str}", {
            f"Δ_A columns {col_str}": delta_col_a,
            f"Δ_B columns {col_str}": delta_col_b,
            f"input_A (fixed)": fixed_col_a,
            f"λ_A": lambda_col_a,
        }, level="crypto")

        # ══ Step 5: Expose Δ ══
        sb.wait(state)
        state.log(f"═══ Exposing Δ via /api/delta ═══", level="crypto")
        with delta_lock:
            delta_store["ready"] = True
            delta_store["data"] = {
                "ready": True,
                "data_party_id": dp_id,
                "columns": cols,
                "dim": dim,
                "delta_col_a": delta_col_a,
                "delta_col_b": delta_col_b,
                "float_col_a": float_col_a,
                "float_col_b": float_col_b,
                "fixed_col_a": fixed_col_a,
                "fixed_col_b": fixed_col_b,
            }
        state.log(f"Δ exposed via GET /api/delta for columns {col_str}")
        state.update(phase="done", status=f"Done — columns {col_str} contributed")


if __name__ == "__main__":
    main()
