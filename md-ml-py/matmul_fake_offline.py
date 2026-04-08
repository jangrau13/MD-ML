"""Fake offline phase for matrix multiplication experiment."""

import time
from md_ml.share import Spdz2kShare64
from md_ml.fake_offline import FakeParty, FakeCircuit

FAKE_OFFLINE_DIR = "../fake-offline-data"
JOB_NAME = "MatMul-py"
DIM = 4096


def main():
    share = Spdz2kShare64

    print(f"Running fake offline phase for {DIM}x{DIM} matmul...")
    t = time.time()

    party = FakeParty(share, JOB_NAME, FAKE_OFFLINE_DIR, n_parties=2)
    circuit = FakeCircuit(share)

    a = circuit.input(0, DIM, DIM)
    b = circuit.input(0, DIM, DIM)
    c = circuit.multiply(a, b)
    d = circuit.output(c)

    circuit.add_endpoint(d)
    circuit.run_offline(party)

    print(f"Fake offline phase done in {(time.time()-t)*1000:.0f} ms")


if __name__ == "__main__":
    main()
