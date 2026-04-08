"""Matrix multiplication benchmark, matching the Rust matmul_bench."""

import time
import numpy as np
from md_ml.linear_algebra import matrix_multiply


def bench(label, sizes, make_data, do_matmul):
    print(f"\n{'=' * 50}")
    print(f"  {label}")
    print(f"{'=' * 50}")
    print(f"  {'Size':>10}  {'Time (ms)':>10}  {'GFLOPS':>10}")
    print(f"  {'-'*10}  {'-'*10}  {'-'*10}")

    for n in sizes:
        a, b = make_data(n)

        # Warmup for small sizes
        if n <= 512:
            do_matmul(a, b, n)

        t = time.time()
        c = do_matmul(a, b, n)
        elapsed_ms = (time.time() - t) * 1000
        gflops = (2.0 * n**3) / (elapsed_ms / 1000.0) / 1e9

        print(f"  {n:>10}  {elapsed_ms:>10.1f}  {gflops:>10.2f}")


def main():
    rng = np.random.default_rng(42)

    # u64 via f64 BLAS (our optimized path)
    bench(
        "uint64_t via f64 BLAS (16-bit split)",
        [512, 1024, 2048, 4096],
        lambda n: (
            rng.integers(0, 2**64, size=n*n, dtype=np.uint64),
            rng.integers(0, 2**64, size=n*n, dtype=np.uint64),
        ),
        lambda a, b, n: matrix_multiply(a, b, n, n, n),
    )

    # u64 via numpy @ (baseline — no BLAS for integer)
    bench(
        "uint64_t numpy @ (baseline, no BLAS)",
        [512, 1024, 2048],
        lambda n: (
            rng.integers(0, 2**64, size=(n, n), dtype=np.uint64),
            rng.integers(0, 2**64, size=(n, n), dtype=np.uint64),
        ),
        lambda a, b, n: a @ b,
    )

    # u128 via f64 BLAS
    import os
    bench(
        "uint128_t via f64 BLAS (16-bit split)",
        [32, 64, 128, 256],
        lambda n: (
            [int.from_bytes(os.urandom(16), "little") for _ in range(n * n)],
            [int.from_bytes(os.urandom(16), "little") for _ in range(n * n)],
        ),
        lambda a, b, n: matrix_multiply(a, b, n, n, n),
    )

    # f64 reference
    bench(
        "double (f64, numpy BLAS)",
        [512, 1024, 2048, 4096],
        lambda n: (rng.random((n, n)), rng.random((n, n))),
        lambda a, b, n: a @ b,
    )


if __name__ == "__main__":
    main()
