"""
Local end-to-end test: runs fake offline + both parties in one process.

Uses a small matrix size to verify correctness without networking.
"""

import threading
import time
import numpy as np
from md_ml.share import Spdz2kShare64
from md_ml.fake_offline import FakeParty, FakeCircuit
from md_ml.protocols import PartyWithFakeOffline, Circuit

FAKE_OFFLINE_DIR = "../fake-offline-data"
JOB_NAME = "test-py"
DIM = 16
PORT = 8080


def run_fake_offline():
    share = Spdz2kShare64
    party = FakeParty(share, JOB_NAME, FAKE_OFFLINE_DIR, n_parties=2)
    circuit = FakeCircuit(share)
    a = circuit.input(0, DIM, DIM)
    b = circuit.input(0, DIM, DIM)
    c = circuit.multiply(a, b)
    d = circuit.output(c)
    circuit.add_endpoint(d)
    circuit.run_offline(party)
    print("[fake-offline] Done")


def run_party(party_id: int, mat_a=None, mat_b=None, expected=None):
    share = Spdz2kShare64
    party = PartyWithFakeOffline(share, party_id, 2, PORT, JOB_NAME, FAKE_OFFLINE_DIR)
    circuit = Circuit(share)

    a = circuit.input(0, DIM, DIM)
    b = circuit.input(0, DIM, DIM)
    c = circuit.multiply(a, b)
    d = circuit.output(c)
    circuit.add_endpoint(d)

    if party_id == 0:
        a.set_input(mat_a, 0, share)
        b.set_input(mat_b, 0, share)

    circuit.read_offline_from_file(party)
    circuit.run_online_with_benchmark(party)
    circuit.print_stats(party)

    if party_id == 0:
        result = d.get_clear(share)
        errors = int(np.sum(result != expected))
        if errors == 0:
            print(f"Party {party_id}: Correctness check PASSED ({len(expected)} elements)")
        else:
            print(f"Party {party_id}: FAILED {errors}/{len(expected)} elements wrong")
            mismatches = np.where(result != expected)[0]
            for idx in mismatches[:5]:
                row, col = divmod(int(idx), DIM)
                print(f"  MISMATCH ({row},{col}): got {result[idx]}, expected {expected[idx]}")


def main():
    # Step 1: fake offline
    print("=== Fake Offline Phase ===")
    run_fake_offline()

    # Step 2: generate inputs
    rng = np.random.default_rng(42)
    mat_a = rng.integers(0, 2**64, size=DIM * DIM, dtype=np.uint64)
    mat_b = rng.integers(0, 2**64, size=DIM * DIM, dtype=np.uint64)
    expected = (mat_a.reshape(DIM, DIM) @ mat_b.reshape(DIM, DIM)).ravel()

    # Step 3: run both parties in parallel threads
    print("\n=== Online Phase ===")
    t0 = threading.Thread(target=run_party, args=(0, mat_a, mat_b, expected))
    t1 = threading.Thread(target=run_party, args=(1,))
    t0.start()
    t1.start()
    t0.join()
    t1.join()

    print("\nDone!")


if __name__ == "__main__":
    main()
