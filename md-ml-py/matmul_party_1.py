"""Matrix multiplication experiment - party 1."""

from md_ml.share import Spdz2kShare64
from md_ml.protocols import PartyWithFakeOffline, Circuit

FAKE_OFFLINE_DIR = "../fake-offline-data"
JOB_NAME = "MatMul-py"
DIM = 4096
PORT = 7070


def main():
    share = Spdz2kShare64

    party = PartyWithFakeOffline(share, 1, 2, PORT, JOB_NAME, FAKE_OFFLINE_DIR)
    circuit = Circuit(share)

    a = circuit.input(0, DIM, DIM)
    b = circuit.input(0, DIM, DIM)
    c = circuit.multiply(a, b)
    d = circuit.output(c)
    circuit.add_endpoint(d)

    circuit.read_offline_from_file(party)
    circuit.run_online_with_benchmark(party)
    circuit.print_stats(party)


if __name__ == "__main__":
    main()
