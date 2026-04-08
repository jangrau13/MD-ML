"""
π_DotProductTrunc — Dot Product with Truncation (Appendix B.2).

Combines π_DotProduct with π_MultTrunc's truncation technique.
Computes z = ⌊(x̄ · ȳ) / 2^d⌋ for fixed-point arithmetic.

Same online communication as plain dot product (1 element per party, 1 round)
but the result is correctly truncated.

The trick: use [λ_{z'}] = 2^d · [λ_z] + [u] instead of [λ_z] as the
output mask. After opening Δ_{z'} and truncating by d bits:
  Δ_z = ⌊Δ_{z'} / 2^d⌋ ≈ ⌊z'/2^d⌋ + λ_z
  result = Δ_z − λ_z ≈ ⌊(x̄ · ȳ) / 2^d⌋

This construction saves one communication round compared to
SPD Z_{2k}+ which needs 2m+1 elements in 2 rounds.
"""

from __future__ import annotations
from .dot_product import dot_product_online
from ..share import ShareConfig


def dot_product_trunc_online(
    share: ShareConfig,
    m: int,
    d_bits: int,
    temp_x_vecs: list[list[int]],
    temp_y_vecs: list[list[int]],
    a_shr_vecs: list[list[int]],
    b_shr_vecs: list[list[int]],
    c_shr_vecs: list[list[int]],
    lzp_shr: list[int],
    lz_shr: list[int],
    temp_xy_vecs: list[list[int]],
    my_id: int,
    output_size: int,
) -> tuple:
    """
    Online phase of π_DotProductTrunc.

    Identical to π_DotProduct but uses [λ_{z'}] instead of [λ_z].
    After opening, the caller truncates Δ_{z'} by d bits and unmasks with λ_z.

    Args:
        (same as dot_product_online, plus:)
        d_bits: fractional bits for truncation
        lzp_shr: [λ_{z'}]^i = 2^d · [λ_z]^i + [u]^i
        lz_shr: [λ_z]^i for final unmasking after truncation

    Returns: (delta_zp_shr, lz_shr) where:
        delta_zp_shr: [Δ_{z'}]^i shares to open and truncate
        lz_shr: [λ_z]^i for unmasking: result = ⌊Δ_{z'}/2^d⌋ − λ_z
    """
    # Same computation as dot_product, but with [λ_{z'}] as mask
    delta_zp = dot_product_online(
        share, m,
        temp_x_vecs, temp_y_vecs,
        a_shr_vecs, b_shr_vecs, c_shr_vecs,
        lzp_shr,  # use λ_{z'} instead of λ_z
        temp_xy_vecs,
        my_id, output_size,
    )

    return delta_zp, lz_shr
