"""
Online phase of π_MultTrunc for a computation party.

Extracted from run_party.py to keep files under 500 LOC.
"""

import time
import json
import numpy as np
from md_ml.linear_algebra import (
    matrix_multiply, matrix_add, matrix_subtract,
    matrix_add_assign, matrix_subtract_assign, matrix_scalar,
)


def run_online_phase(
    sb, state, share, net, my_id, other_id, dim, d_bits, size,
    a_shr, a_mac, b_shr, b_mac, c_shr, c_mac,
    lzp_shr, lzp_mac, lz_shr, lz_mac,
    delta_x_pub, delta_y_pub,
    delta_a_semi, delta_b_semi,
    global_key_shr,
    bytes_sent,
    deltas_lock, deltas_store,
):
    """Run the online phase steps. Returns (result, bytes_sent)."""
    sm = share.semi_mask

    # Wait for Δ from data parties
    sb.wait(state)
    state.log("Waiting for Δ_A, Δ_B from data parties via /api/deltas...", level="crypto")
    while True:
        with deltas_lock:
            if deltas_store["ready"]:
                delta_data = deltas_store["data"]
                break
        time.sleep(0.5)

    if share.clear_dtype is not None:
        delta_a_clear = np.array([int(x) for x in delta_data["delta_a"]], dtype=share.clear_dtype)
        delta_b_clear = np.array([int(x) for x in delta_data["delta_b"]], dtype=share.clear_dtype)
    else:
        delta_a_clear = [int(x) & share.clear_mask for x in delta_data["delta_a"]]
        delta_b_clear = [int(x) & share.clear_mask for x in delta_data["delta_b"]]
    delta_a_semi = share.widen_clear_to_semi(delta_a_clear)
    delta_b_semi = share.widen_clear_to_semi(delta_b_clear)

    state.log(f"Received Δ_A, Δ_B ({size} elements each)", {
        "Δ_A": delta_a_semi,
        "Δ_B": delta_b_semi,
    }, level="crypto")

    # temp_x, temp_y
    sb.wait(state)
    state.log("═══ Online Phase: π_MultTrunc ═══", level="crypto")
    temp_x = matrix_add(delta_a_semi, delta_x_pub, mask=sm)
    state.log("temp_x = Δ_A + δ_x = A + a", {"temp_x": temp_x}, level="crypto")

    sb.wait(state)
    temp_y = matrix_add(delta_b_semi, delta_y_pub, mask=sm)
    state.log("temp_y = Δ_B + δ_y = B + b", {"temp_y": temp_y}, level="crypto")

    sb.wait(state)
    t0 = time.time()
    temp_xy = matrix_multiply(temp_x, temp_y, dim, dim, dim, mask=sm)
    state.log(f"Matmul 1/5: temp_xy = (A+a)·(B+b) [{(time.time()-t0)*1000:.1f} ms]", {
        "temp_xy": temp_xy,
    }, level="crypto")

    sb.wait(state)
    delta_zp_shr = matrix_add(c_shr, lzp_shr, mask=sm)
    state.log("[Δ_{z'}]^i = [c]^i + [λ_{z'}]^i  (using λ_{z'}, NOT λ_z!)", {
        "[c]^i": c_shr, "[λ_{z'}]^i": lzp_shr, "[Δ_{z'}]^i": delta_zp_shr,
    }, level="crypto")

    sb.wait(state)
    t0 = time.time()
    delta_zp_shr = matrix_subtract_assign(delta_zp_shr, matrix_multiply(a_shr, temp_y, dim, dim, dim, mask=sm), mask=sm)
    state.log(f"Matmul 2/5: [Δ_{{z'}}] -= [a]·temp_y [{(time.time()-t0)*1000:.1f} ms]", level="crypto")

    sb.wait(state)
    t0 = time.time()
    delta_zp_shr = matrix_subtract_assign(delta_zp_shr, matrix_multiply(temp_x, b_shr, dim, dim, dim, mask=sm), mask=sm)
    state.log(f"Matmul 3/5: [Δ_{{z'}}] -= temp_x·[b] [{(time.time()-t0)*1000:.1f} ms]", level="crypto")

    sb.wait(state)
    if my_id == 0:
        delta_zp_shr = matrix_add_assign(delta_zp_shr, temp_xy, mask=sm)
        state.log("P_0: [Δ_{z'}]^0 += temp_xy", {"[Δ_{z'}]^0": delta_zp_shr}, level="crypto")
    else:
        state.log("P_1: skip (only P_0 adds public term)")

    # MAC computation
    sb.wait(state)
    state.log("═══ MAC for [Δ_{z'}] ═══", level="crypto")
    gk = share.widen_global_to_semi(global_key_shr)
    delta_zp_mac = matrix_scalar(temp_xy, gk, mask=sm)
    delta_zp_mac = matrix_add_assign(delta_zp_mac, c_mac, mask=sm)
    delta_zp_mac = matrix_add_assign(delta_zp_mac, lzp_mac, mask=sm)
    state.log(f"[Δ_{{z'}}_mac]^{my_id}", level="crypto")

    sb.wait(state)
    t0 = time.time()
    delta_zp_mac = matrix_subtract_assign(delta_zp_mac, matrix_multiply(a_mac, temp_y, dim, dim, dim, mask=sm), mask=sm)
    state.log(f"Matmul 4/5: MAC correction [{(time.time()-t0)*1000:.1f} ms]", level="crypto")

    sb.wait(state)
    t0 = time.time()
    delta_zp_mac = matrix_subtract_assign(delta_zp_mac, matrix_multiply(temp_x, b_mac, dim, dim, dim, mask=sm), mask=sm)
    state.log(f"Matmul 5/5: MAC correction [{(time.time()-t0)*1000:.1f} ms]", level="crypto")

    # Exchange [Δ_{z'}]
    sb.wait(state)
    state.log("═══ Communication Round ═══", level="crypto")
    send_data = share.semi_to_bytes(delta_zp_shr)
    nbytes = size * share.semi_bytes
    t0 = time.time()
    recv_data = net.send_recv_concurrent(other_id, send_data, nbytes)
    state.log(f"Exchanged [Δ_{{z'}}] ({len(send_data):,} bytes, {(time.time()-t0)*1000:.1f} ms)", level="crypto")
    bytes_sent += len(send_data)
    state.update(bytes_sent=bytes_sent)

    # Reconstruct Δ_{z'}
    sb.wait(state)
    other_dzp = share.semi_from_bytes(recv_data, size)
    dzp_combined = matrix_add_assign(other_dzp, delta_zp_shr, mask=sm)
    delta_zprime = share.remove_upper_bits(dzp_combined)
    state.log("Δ_{z'} = Σ[Δ_{z'}]^i mod 2^k", {"Δ_{z'}": delta_zprime}, level="crypto")

    # Truncation
    sb.wait(state)
    state.log(f"═══ π_MultTrunc: TRUNCATION by 2^{d_bits} ═══", level="crypto")
    if isinstance(delta_zprime, np.ndarray):
        delta_z_trunc = delta_zprime >> delta_zprime.dtype.type(d_bits)
    else:
        delta_z_trunc = [v >> d_bits for v in delta_zprime]
    state.log(f"Δ_z (after truncation)", {f"Δ_z = Δ_{{z'}} >> {d_bits}": delta_z_trunc}, level="crypto")

    # Exchange [λ_z]
    sb.wait(state)
    state.log("═══ Output Phase ═══", level="crypto")
    send_data = share.semi_to_bytes(lz_shr)
    nbytes = size * share.semi_bytes
    t0 = time.time()
    recv_data = net.send_recv_concurrent(other_id, send_data, nbytes)
    state.log(f"[λ_z] exchanged ({(time.time()-t0)*1000:.1f} ms)", level="crypto")
    bytes_sent += len(send_data)
    state.update(bytes_sent=bytes_sent)

    sb.wait(state)
    other_lz = share.semi_from_bytes(recv_data, size)
    lambda_z_clear = matrix_add_assign(other_lz, lz_shr, mask=sm)
    state.log("λ_z = Σ[λ_z]^i reconstructed", {"λ_z": lambda_z_clear}, level="crypto")

    sb.wait(state)
    state.log(f"═══ Final Result ═══", level="crypto")
    if isinstance(delta_z_trunc, np.ndarray):
        result_raw = delta_z_trunc - lambda_z_clear
    else:
        result_raw = matrix_subtract(delta_z_trunc, lambda_z_clear, mask=share.clear_mask)
    state.log("result = Δ_z − λ_z = ⌊(A·B) / 2^d⌋", {
        "Δ_z": delta_z_trunc, "λ_z": lambda_z_clear, "result": result_raw,
    }, level="crypto")

    sb.wait(state)
    result = share.remove_upper_bits(result_raw)
    state.log(f"Final result ∈ Z_{{2^{share.k_bits}}}", {"result": result}, level="crypto")

    return result, bytes_sent
