"""
π_DotProduct — Constant-Communication Vector Dot Product (Procedure 2, Section 4.2).

For input vectors x̄, ȳ of length m, computes z = x̄ · ȳ = Σ_{i=1}^{m} x[i]·y[i].

Key achievement: online communication is 1 ring element per party,
INDEPENDENT of vector length m. This is the first time this is achieved
in the maliciously secure dishonest majority setting.

Preprocessing (per dot product):
  1. F_Prep.Rand → [λ_z] (one random shared value for output)
  2. F_Prep.Triple × m → {([a[i]], [b[i]], [c[i]])} (m triples)
  3. Compute δ̄_x = ā − λ̄_x, δ̄_y = b̄ − λ̄_y, open δ̄_x, δ̄_y

Online:
  1. [Δ_z] = Σ_{i=1}^{m} ( (Δ_x[i] + δ_x[i])(Δ_y[i] + δ_y[i])
             − (Δ_y[i] + δ_y[i])[a[i]] − (Δ_x[i] + δ_x[i])[b[i]]
             + [c[i]] ) + [λ_z]
  2. Open Δ_z

Communication: each party sends 1 element of Z_{2^{k+s}} (total).
"""

from __future__ import annotations
from ..share import ShareConfig


def dot_product_online(
    share: ShareConfig,
    m: int,
    temp_x_vecs: list[list[int]],
    temp_y_vecs: list[list[int]],
    a_shr_vecs: list[list[int]],
    b_shr_vecs: list[list[int]],
    c_shr_vecs: list[list[int]],
    lambda_z_shr: list[int],
    temp_xy_vecs: list[list[int]],
    my_id: int,
    output_size: int,
) -> list[int]:
    """
    Online phase of π_DotProduct.

    For each output element j, compute:
      [Δ_z[j]] = Σ_{i=0}^{m-1} (
        temp_xy[i][j]
        − temp_y[i][j] · [a[i][j]]
        − temp_x[i][j] · [b[i][j]]
        + [c[i][j]]
      ) + [λ_z[j]]

    Only Party 0 adds the temp_xy terms.

    Args:
        m: dot product length (number of element-wise products to sum)
        temp_x_vecs: m vectors of (Δ_x[i] + δ_x[i]) — public
        temp_y_vecs: m vectors of (Δ_y[i] + δ_y[i]) — public
        a_shr_vecs: m vectors of [a[i]]^party_id — secret shares
        b_shr_vecs: m vectors of [b[i]]^party_id — secret shares
        c_shr_vecs: m vectors of [c[i]]^party_id — secret shares
        lambda_z_shr: [λ_z]^party_id (one per output element)
        temp_xy_vecs: m vectors of temp_x[i]·temp_y[i] — public
        my_id: party ID (0 or 1)
        output_size: number of output elements

    Returns: [Δ_z]^party_id shares (to be opened by exchanging with other party)
    """
    sm = share.semi_mask

    delta_z = list(lambda_z_shr)  # start with [λ_z]

    for i in range(m):
        for j in range(output_size):
            # Beaver multiplication: temp_xy - temp_y·a - temp_x·b + c
            term = c_shr_vecs[i][j]
            term = (term - temp_y_vecs[i][j] * a_shr_vecs[i][j]) & sm
            term = (term - temp_x_vecs[i][j] * b_shr_vecs[i][j]) & sm
            # Only Party 0 adds the public product
            if my_id == 0:
                term = (term + temp_xy_vecs[i][j]) & sm
            delta_z[j] = (delta_z[j] + term) & sm

    return delta_z
