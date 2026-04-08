"""Matrix multiplication experiment - party 0."""

import sys
import time
import numpy as np
from md_ml.share import Spdz2kShare64
from md_ml.protocols import PartyWithFakeOffline, Circuit

FAKE_OFFLINE_DIR = "../fake-offline-data"
JOB_NAME = "MatMul-py"
DIM = 4096
PORT = 7070


def main():
    share = Spdz2kShare64

    rng = np.random.default_rng()
    mat_a = rng.integers(0, 2**64, size=DIM * DIM, dtype=np.uint64)
    mat_b = rng.integers(0, 2**64, size=DIM * DIM, dtype=np.uint64)

    # Compute expected result locally (wrapping u64 arithmetic via numpy)
    print(f"Computing {DIM}x{DIM} reference matmul...", flush=True)
    t = time.time()
    expected = (mat_a.reshape(DIM, DIM).astype(np.uint64) @ mat_b.reshape(DIM, DIM).astype(np.uint64)).ravel()
    print(f"Reference matmul done in {(time.time()-t)*1000:.0f} ms", flush=True)

    party = PartyWithFakeOffline(share, 0, 2, PORT, JOB_NAME, FAKE_OFFLINE_DIR)
    circuit = Circuit(share)

    a = circuit.input(0, DIM, DIM)
    b = circuit.input(0, DIM, DIM)
    c = circuit.multiply(a, b)
    d = circuit.output(c)
    circuit.add_endpoint(d)

    a.set_input(mat_a, 0, share)
    b.set_input(mat_b, 0, share)

    circuit.read_offline_from_file(party)
    circuit.run_online_with_benchmark(party)
    circuit.print_stats(party)

    # Correctness check
    result = d.get_clear(share)
    if isinstance(result, np.ndarray):
        errors = int(np.sum(result != expected))
        if errors > 0 and errors < 10:
            mismatches = np.where(result != expected)[0]
            for idx in mismatches[:10]:
                row, col = divmod(int(idx), DIM)
                print(f"MISMATCH at ({row}, {col}): got {result[idx]}, expected {expected[idx]}")
    else:
        errors = sum(1 for g, e in zip(result, expected) if g != e)

    if errors == 0:
        print(f"Correctness check PASSED: all {len(expected)} elements match")
    else:
        print(f"Correctness check FAILED: {errors}/{len(expected)} elements wrong")


if __name__ == "__main__":
    main()
