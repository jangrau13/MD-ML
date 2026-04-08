#!/usr/bin/env python3
"""
End-to-end test: run secure matrix multiplication without the dashboard.

Directly drives the data party and computation party APIs to:
1. Configure all parties with dim, d, k
2. Step data parties through all steps
3. Collect deltas from data parties, assemble full matrices
4. Send deltas to computation parties
5. Step computation parties through all steps
6. Compare MPC result against plaintext reference

Usage:
    # Start the backend services first:
    docker compose up -d party0 party1 dp0 dp1

    # Then run this test:
    uv run python test_e2e.py --dim 2 --d 1 --k 5
"""

import argparse
import json
import sys
import time
import urllib.request
import urllib.error


def api_get(url: str, timeout: int = 5):
    resp = urllib.request.urlopen(url, timeout=timeout)
    return json.loads(resp.read().decode())


def api_post(url: str, data: dict | None = None, timeout: int = 10):
    body = json.dumps(data or {}).encode()
    req = urllib.request.Request(
        url, data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    resp = urllib.request.urlopen(req, timeout=timeout)
    return json.loads(resp.read().decode())


def wait_for_service(url: str, label: str, retries: int = 30):
    for _ in range(retries):
        try:
            api_get(url)
            return True
        except Exception:
            time.sleep(1)
    print(f"ERROR: {label} not reachable at {url}")
    return False


def step_until_done(urls: list[str], label: str, max_steps: int = 100):
    """Step all URLs in lockstep until all are done."""
    for step_num in range(max_steps):
        # Step all
        for url in urls:
            try:
                api_post(f"{url}/api/step")
            except Exception as e:
                print(f"  Warning: step failed for {url}: {e}")

        time.sleep(0.3)

        # Check states
        states = []
        for url in urls:
            try:
                s = api_get(f"{url}/api/state")
                states.append(s)
            except Exception:
                states.append(None)

        phases = [s.get("phase", "?") if s else "?" for s in states]
        steps = [f"{s.get('current_step', '?')}/{s.get('total_steps', '?')}" if s else "?" for s in states]

        all_done = all(p == "done" for p in phases)
        print(f"  {label} step {step_num}: phases={phases} steps={steps}")

        if all_done:
            return True

        # Check for errors
        for i, s in enumerate(states):
            if s and "error" in s.get("phase", ""):
                print(f"  ERROR in {urls[i]}: {s.get('status', '')}")
                return False

    print(f"  {label} did not finish in {max_steps} steps")
    return False


def main():
    parser = argparse.ArgumentParser(description="E2E test for MD-ML")
    parser.add_argument("--dim", type=int, default=2)
    parser.add_argument("--d", type=int, default=1)
    parser.add_argument("--k", type=int, default=5)
    parser.add_argument("--party0", default="http://localhost:8081")
    parser.add_argument("--party1", default="http://localhost:8082")
    parser.add_argument("--dp0", default="http://localhost:8090")
    parser.add_argument("--dp1", default="http://localhost:8091")
    args = parser.parse_args()

    dim, d, k = args.dim, args.d, args.k
    mask = (1 << k) - 1
    size = dim * dim

    comp_urls = [args.party0, args.party1]
    dp_urls = [args.dp0, args.dp1]
    all_urls = comp_urls + dp_urls

    print(f"=== MD-ML E2E Test: {dim}×{dim}, d={d}, k={k} (mod 2^{k}={1<<k}) ===")
    print()

    # ── 1. Wait for services ──
    print("Waiting for services...")
    for url in all_urls:
        if not wait_for_service(f"{url}/api/state", url):
            sys.exit(1)
    print("All services reachable.\n")

    # ── 2. Trigger MP-SPDZ preprocessing (both parties simultaneously) ──
    mpspdz_urls = ["http://localhost:5000", "http://localhost:5001"]
    print("Triggering MP-SPDZ preprocessing...")
    mpspdz_config = {"k": k, "s": k, "protocol": "spdz2k", "program": "bench_simple"}
    for url in mpspdz_urls:
        try:
            api_post(f"{url}/api/run", mpspdz_config)
            print(f"  {url} triggered")
        except Exception as e:
            print(f"  {url} trigger failed: {e}")

    print("Waiting for MP-SPDZ to complete...")
    for attempt in range(120):
        time.sleep(1)
        try:
            states = [api_get(f"{url}/api/status") for url in mpspdz_urls]
            phases = [s.get("phase", "?") for s in states]
            if attempt % 10 == 0:
                print(f"  MP-SPDZ phases: {phases}")
            if all(p == "done" for p in phases):
                print(f"  MP-SPDZ done in {states[0].get('elapsed_ms', '?')}ms")
                break
            if any(p == "error" for p in phases):
                for s in states:
                    if s.get("phase") == "error":
                        print(f"  MP-SPDZ error: {s.get('error', '?')}")
                break
        except Exception:
            pass
    print()

    # ── 3. Configure all parties ──
    print("Configuring all parties...")
    config = {"dim": dim, "d": d, "k": k}
    for url in all_urls:
        try:
            api_post(f"{url}/api/configure", config)
            print(f"  {url} configured")
        except Exception as e:
            print(f"  {url} configure failed: {e}")
    print()

    # ── 4. Step all parties in parallel until DPs are done ──
    # DPs and comp parties run concurrently:
    #   - Comp parties do preprocessing (steps 0-7: MP-SPDZ trigger, read shares, expose λ)
    #   - DPs do preprocessing (steps 0-1: generate floats, FPA encode)
    #   - Once comp parties expose λ (step 5+), DPs can proceed (steps 2-5: collect λ, compute Δ)
    print("Running all parties in parallel...")
    all_party_urls = comp_urls + dp_urls
    for step_num in range(200):
        # Step all parties
        for url in all_party_urls:
            try:
                api_post(f"{url}/api/step")
            except Exception:
                pass
        time.sleep(0.3)

        # Check DP states
        dp_states = []
        for url in dp_urls:
            try:
                dp_states.append(api_get(f"{url}/api/state"))
            except Exception:
                dp_states.append(None)

        dp_phases = [s.get("phase", "?") if s else "?" for s in dp_states]
        dp_steps = [f"{s.get('current_step', '?')}/{s.get('total_steps', '?')}" if s else "?" for s in dp_states]

        # Also check comp party progress
        comp_states = []
        for url in comp_urls:
            try:
                comp_states.append(api_get(f"{url}/api/state"))
            except Exception:
                comp_states.append(None)
        comp_steps = [f"{s.get('current_step', '?')}/{s.get('total_steps', '?')}" if s else "?" for s in comp_states]

        if step_num % 5 == 0:
            print(f"  step {step_num}: DPs={dp_phases} {dp_steps}  Comp={comp_steps}")

        if all(p == "done" for p in dp_phases):
            print(f"  All data parties done at step {step_num}")
            break
    else:
        print("  Data parties did not finish!")
        sys.exit(1)
    print()

    # ── 4. Collect deltas from data parties ──
    print("Collecting deltas from data parties...")
    float_a = [0.0] * size
    float_b = [0.0] * size
    delta_a = ["0"] * size
    delta_b = ["0"] * size
    fixed_a = ["0"] * size
    fixed_b = ["0"] * size

    for dp_url in dp_urls:
        data = api_get(f"{dp_url}/api/delta")
        if not data.get("ready"):
            print(f"  WARNING: {dp_url} delta not ready")
            continue
        cols = data["columns"]
        print(f"  {dp_url}: columns {cols}, {len(data['delta_col_a'])} elements")
        idx = 0
        for row in range(dim):
            for col in cols:
                flat = row * dim + col
                float_a[flat] = data["float_col_a"][idx]
                float_b[flat] = data["float_col_b"][idx]
                delta_a[flat] = data["delta_col_a"][idx]
                delta_b[flat] = data["delta_col_b"][idx]
                fixed_a[flat] = data["fixed_col_a"][idx]
                fixed_b[flat] = data["fixed_col_b"][idx]
                idx += 1

    print(f"  Assembled A (float): {float_a}")
    print(f"  Assembled B (float): {float_b}")
    print(f"  Assembled Δ_A: {delta_a}")
    print(f"  Assembled Δ_B: {delta_b}")
    print()

    # ── 5. Compute plaintext reference ──
    print("Computing plaintext reference...")
    ref_float = []
    ref_fixed = []
    for i in range(dim):
        for j in range(dim):
            fsum = 0.0
            isum = 0
            for m in range(dim):
                fsum += float_a[i * dim + m] * float_b[m * dim + j]
                isum += int(fixed_a[i * dim + m]) * int(fixed_b[m * dim + j])
            ref_float.append(round(fsum * 100) / 100)
            ref_fixed.append(str((isum >> d) & mask))
    print(f"  Float reference C = A×B: {ref_float}")
    print(f"  Fixed reference (truncated): {ref_fixed}")
    print()

    # ── 6. Send deltas to computation parties ──
    print("Sending deltas to computation parties...")
    delta_payload = {"delta_a": delta_a, "delta_b": delta_b}
    for url in comp_urls:
        try:
            api_post(f"{url}/api/deltas", delta_payload)
            print(f"  {url} deltas sent")
        except Exception as e:
            print(f"  {url} deltas failed: {e}")
    print()

    # ── 7. Step computation parties until done ──
    print("Running computation parties...")
    if not step_until_done(comp_urls, "Comp"):
        print("Computation parties failed!")
        sys.exit(1)
    print()

    # ── 8. Get MPC result ──
    print("Fetching MPC result...")
    result_state = api_get(f"{comp_urls[0]}/api/state")
    result_raw = result_state.get("result")
    if not result_raw:
        print("ERROR: No result from Party 0")
        sys.exit(1)
    mpc_result = json.loads(result_raw)
    print(f"  MPC result: {mpc_result}")
    print(f"  Reference:  {ref_fixed}")
    print()

    # ── 9. Compare ──
    print("=== Comparison ===")
    all_match = True
    for i in range(size):
        mpc_val = int(mpc_result[i]) & mask
        ref_val = int(ref_fixed[i]) & mask
        match = "✓" if mpc_val == ref_val else "✗"
        if mpc_val != ref_val:
            all_match = False
        row, col = i // dim, i % dim
        print(f"  C[{row},{col}]: MPC={mpc_val}, ref={ref_val}  {match}")

    print()
    if all_match:
        print("=== ALL MATCH — TEST PASSED ===")
    else:
        print("=== MISMATCH — TEST FAILED ===")
        # Show float comparison for context
        print(f"  Float A: {float_a}")
        print(f"  Float B: {float_b}")
        print(f"  Float C: {ref_float}")
        sys.exit(1)


if __name__ == "__main__":
    main()
