# MD-ML: Open Issues & Next Steps

## Critical: MPC Result Does Not Match Reference

**Symptom:** With `k=5, d=1, dim=2`, the reference fixed-point matmul gives `[11, 15, 6, 9]` (floats `[5.5, 7.5, 3.0, 4.5]`) but the MPC protocol produces `[21, 1, 21, 29]` (floats `[-5.5, 0.5, -5.5, -1.5]`).

**Likely root cause:** MP-SPDZ generates **scalar** Beaver triples via `sint.get_random_triple()`, but the online phase in `docker/online_phase.py` uses them for **matrix** multiplication. A matrix Beaver triple requires `c = matmul(a, b)`, not `c = a * b` element-wise. Scalar triples only satisfy `c[i] = a[i] * b[i]`, which is wrong for matrix products where `c[i,j] = Σ_k a[i,k] * b[k,j]`.

**Investigation needed:**
1. Read `docker/online_phase.py` to understand how triples are consumed
2. Verify whether the protocol expects scalar or matrix triples
3. Check if the `bench_simple.mpc` program needs to generate matrix triples (n³ scalar triples arranged as a matrix product) instead of n² independent scalar triples
4. Verify the truncation masks (`λ_z`, `λ_{z'}`) from `generate_multtrunc_masks()` are correct for `k=5, s=64`

## Fixed Recently

- **λ_A = λ_B duplicate bug** — `read_input_masks` was reading from the same file offset twice. Fixed with `mask_offset` parameter.
- **All-zero matrices** — DP loop cleared delta store before coordinator could read. Fixed by clearing only after new `wait_for_config()` returns.
- **SQLite removed** — Backend uses in-memory state. Reset via `POST /api/reset`.
- **Backend loop** — Both `run_party.py` and `run_data_party.py` loop after completion, waiting for next `POST /api/configure`.
- **MP-SPDZ multi-ring** — Compiled binaries for `RING_SIZE` 5, 10, 32, 64. Runtime selects correct binary.
- **MP-SPDZ s=64** — Security parameter is always 64 regardless of k. `ShareConfig` uses `s=64` for reading Player-Data.
- **Persistence file** — `bench_simple.mpc` uses `sint.write_to_file()` to persist triples and input masks. Reader parses from `Persistence/Transactions-P<id>.data`.
- **Variable Board** — Shows actual `k` from config instead of hardcoded 64.
- **MPC result as float** — Converts fixed-point integers back via signed interpretation and `/ 2^d`.
- **Preview matrices** — A, B, C shown immediately after DPs generate floats, before protocol completes.
- **Reference C** — Now computed via fixed-point arithmetic (encode, matmul mod 2^k, truncate) instead of plain float multiply.

## Dashboard UX

- [ ] Step-all progress bar works but hasn't been tested after the reset changes
- [ ] "New Computation" button sends `POST /api/reset` to all backends — verify it works end-to-end
- [ ] Session history is in-memory (lost on dashboard restart) — acceptable for now
- [ ] ConfigPanel `d` options should be constrained to `d < k`

## Protocol / MP-SPDZ Integration

- [ ] **Input masks from MP-SPDZ** — Currently using `sint.get_random_input_mask_for(0)` which generates masks where player 0 is the input owner. Verify this is correct for the data party flow where DPs reconstruct λ from both computation parties' shares.
- [ ] **Persistence format** — Verify the `sint.write_to_file()` binary layout matches what `mpspdz_reader.py` parses (header skip, element size `ceil((k+s)/8)`, value then MAC ordering).
- [ ] **Stale volumes** — Docker volumes persist across runs. Old preprocessing data from different `k` values may cause silent corruption. Consider clearing volumes on configure, or namespacing by config hash.
