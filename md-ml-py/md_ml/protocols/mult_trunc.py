"""
π_MultTrunc — Multiplication with Truncation (Procedure 1, Section 4.1).

For a multiply-then-truncate gate with input wires x, y and output z,
computes z = ⌊x·y / 2^d⌋ where d is the number of fractional bits.

Key insight: truncation is integrated into multiplication at NO extra
communication cost. The online cost is identical to plain multiplication.

Preprocessing phase:
  1. F_Prep.Triple → ([a], [b], [c]) where c = a·b
  2. Compute [δ_x] = [a] − [λ_x], [δ_y] = [b] − [λ_y], open δ_x, δ_y
  3. F_edaBits(k-d) → [λ_z] with bit decomposition
  4. F_edaBits(d) → [u] with bit decomposition
  5. [λ_{z'}] = 2^d · [λ_z] + [u]

Online phase:
  1. [Δ_{z'}] = (Δ_x+δ_x)(Δ_y+δ_y) − (Δ_y+δ_y)[a] − (Δ_x+δ_x)[b] + [c] + [λ_{z'}]
  2. Open Δ_{z'} to get it in the clear
  3. Δ_z = ⌊Δ_{z'} / 2^d⌋ (local truncation!)
  4. result = Δ_z − λ_z = ⌊(x·y) / 2^d⌋
"""

from __future__ import annotations
from .edabits import generate_edabits
from ..share import ShareConfig


def generate_multtrunc_masks(
    share: ShareConfig,
    d_bits: int,
    size: int,
    my_id: int,
    alpha_share: int = 0,
) -> tuple:
    """
    Generate the truncation pair ([λ_{z'}], [λ_z]) for π_MultTrunc.

    From Procedure 1, preprocessing steps 3-5:
      3. F_edaBits(k-d) → [λ_z] and bit decomposition {[λ_{z,i}]_2}
      4. F_edaBits(d) → [u] and bit decomposition {[u_i]_2}
      5. [λ_{z'}] = 2^d · [λ_z] + [u]

    The crucial property: after opening Δ_{z'} = z' + λ_{z'} and truncating
    by d bits, we get Δ_z ≈ z'/2^d + λ_z (because u < 2^d disappears
    in the truncation). Then result = Δ_z − λ_z ≈ z'/2^d.

    Returns: (lz_shr, lz_mac, lzp_shr, lzp_mac, lz_bits, u_bits)
    """
    k = share.k_bits
    sm = share.semi_mask
    kd = k - d_bits

    # Step 3: F_edaBits(k-d) → [λ_z]
    lz_shr, lz_mac, lz_bits, lz_bit_macs = generate_edabits(
        share, kd, size, my_id, alpha_share
    )

    # Step 4: F_edaBits(d) → [u]
    u_shr, u_mac, u_bits, u_bit_macs = generate_edabits(
        share, d_bits, size, my_id, alpha_share
    )

    # Step 5: [λ_{z'}] = 2^d · [λ_z] + [u]
    scale = 1 << d_bits
    lzp_shr = [((lz * scale) + u) & sm for lz, u in zip(lz_shr, u_shr)]
    lzp_mac = [((lm * scale) + um) & sm for lm, um in zip(lz_mac, u_mac)]

    return lz_shr, lz_mac, lzp_shr, lzp_mac, lz_bits, u_bits


def multtrunc_online(
    share: ShareConfig,
    d_bits: int,
    delta_a_semi: list[int],
    delta_b_semi: list[int],
    delta_x_pub: list[int],
    delta_y_pub: list[int],
    a_shr: list[int],
    b_shr: list[int],
    c_shr: list[int],
    lzp_shr: list[int],
    lz_shr: list[int],
    my_id: int,
    dim: int,
) -> tuple:
    """
    Online phase of π_MultTrunc for matrix multiplication.

    This computes:
      temp_x = Δ_A + δ_x = A + a
      temp_y = Δ_B + δ_y = B + b
      [Δ_{z'}] = temp_x·temp_y − temp_y·[a] − temp_x·[b] + [c] + [λ_{z'}]
      (Party 0 adds temp_x·temp_y; Party 1 doesn't)

    After opening Δ_{z'} and truncating:
      Δ_z = ⌊Δ_{z'} / 2^d⌋
      result = Δ_z − λ_z

    Returns: (delta_zp_shr, lz_shr) — the masked product shares and λ_z for unmasking
    """
    from ..linear_algebra import (
        matrix_multiply, matrix_add, matrix_add_assign,
        matrix_subtract_assign,
    )

    sm = share.semi_mask

    # temp_x = Δ_A + δ_x, temp_y = Δ_B + δ_y
    temp_x = matrix_add(delta_a_semi, delta_x_pub, mask=sm)
    temp_y = matrix_add(delta_b_semi, delta_y_pub, mask=sm)

    # temp_xy = temp_x × temp_y (public matrix multiply)
    temp_xy = matrix_multiply(temp_x, temp_y, dim, dim, dim, mask=sm)

    # [Δ_{z'}] = [c] + [λ_{z'}]
    delta_zp = matrix_add(c_shr, lzp_shr, mask=sm)

    # [Δ_{z'}] -= [a] × temp_y
    delta_zp = matrix_subtract_assign(
        delta_zp, matrix_multiply(a_shr, temp_y, dim, dim, dim, mask=sm), mask=sm
    )

    # [Δ_{z'}] -= temp_x × [b]
    delta_zp = matrix_subtract_assign(
        delta_zp, matrix_multiply(temp_x, b_shr, dim, dim, dim, mask=sm), mask=sm
    )

    # Party 0 adds temp_xy
    if my_id == 0:
        delta_zp = matrix_add_assign(delta_zp, temp_xy, mask=sm)

    return delta_zp, lz_shr
