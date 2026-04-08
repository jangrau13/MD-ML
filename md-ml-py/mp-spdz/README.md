# MP-SPDZ Preprocessing Benchmark

Benchmarks the **real** SPDZ-2k preprocessing phase (Beaver triple generation)
using [MP-SPDZ](https://github.com/data61/MP-SPDZ), replacing the trusted
dealer with actual OT-based cryptographic protocols.

## Architecture

```
Previous (trusted dealer):        Now (MP-SPDZ):
                                  
  Dealer (trusted)                Party 0 ←──OT──→ Party 1
     │                               │                 │
     ├── shares ──→ Party 0          [a]^0            [a]^1
     └── shares ──→ Party 1          [b]^0            [b]^1
                                     [c]^0            [c]^1
  Dealer sees a, b, c             Nobody sees a, b, c
```

The dashboard/client is **not involved** in the offline phase — it only
participates in the input phase (receiving λ to compute Δ = input + λ).

## Protocols Available

| Protocol | Executable | Security | Method |
|----------|-----------|----------|--------|
| `spdz2k` | `spdz2k-party.x` | Malicious | OT + MAC sacrifice |
| `semi2k` | `semi2k-party.x` | Semi-honest | OT only (no MACs) |

## Quick Start

```bash
# Build and run the benchmark (both parties)
docker compose up party0-mpspdz party1-mpspdz

# Check status
curl http://localhost:5000/api/status   # party 0
curl http://localhost:5001/api/status   # party 1
```

## Configuration

Environment variables in `docker-compose.yml`:

| Variable | Default | Description |
|----------|---------|-------------|
| `PROTOCOL` | `spdz2k` | Protocol: `spdz2k` (malicious) or `semi2k` (semi-honest) |
| `PROGRAM` | `bench_simple` | MPC program to run |
| `K_BITS` | `64` | Ring size (value bits) |
| `S_BITS` | `64` | Security parameter (MAC bits) |

## Programs

- `bench_simple` — Generate 10,000 scalar Beaver triples. Quick throughput test.
- `bench_matmul_triples` — Generate full preprocessing for n×n matrix multiply
  (input masks, matrix triple, edaBits for truncation).

## Reading MP-SPDZ Output

The `md_ml.mpspdz_reader` module reads MP-SPDZ's `Player-Data/` files directly:

```python
from md_ml.share import make_share_config
from md_ml.mpspdz_reader import MpSpdzReader

share = make_share_config(64)
reader = MpSpdzReader(share, n_parties=2, party_id=0)

# Read 100 Beaver triples
triples = reader.read_triples(100)
for t in triples:
    print(f"a={t.a_val}, b={t.b_val}, c={t.c_val}")

# Read as arrays (matching our protocol format)
a_val, a_mac, b_val, b_mac, c_val, c_mac = reader.read_triples_as_arrays(100)
```

## File Format

MP-SPDZ stores SPDZ-2k shares in `Player-Data/2-Z{k},{s}-{k}/`:

```
Triples-Z64,64-P0     # Party 0's triple shares
Triples-Z64,64-P1     # Party 1's triple shares
Inputs-Z64,64-P0-0    # Party 0's input masks (from player 0)
```

Each file has a binary header (type string + ring spec + MAC key),
followed by packed shares in little-endian format.
For k=64, s=64: each ring element is 16 bytes (Z_{2^128}).
