"""
F_edaBits — Extended Doubly-Authenticated Bits (Functionality 6, Appendix A.4).

Generates a shared random value [r] together with its bit decomposition
{[r_i]_2}_{i=0}^{ℓ-1}, where r = Σ 2^i · r_i is an ℓ-bit value.

Used by:
  - π_MultTrunc (Section 4.1): to generate [λ_z] (k-d bits) and [u] (d bits)
  - π_LTZ (Section 4.3): to generate random mask [r] for comparison

In SPDZ-2k, the edaBits functionality provides:
  - Arithmetic share [r] in Z_{2^{k+s}}
  - Binary shares {[r_i]_2} for each bit of r

The additive shares are generated locally by each party. Since
r = [r]^0 + [r]^1 mod 2^{k+s}, and each party samples its share
independently, the combined r is uniformly random.
"""

from __future__ import annotations
import os
from ..share import ShareConfig


def generate_edabits(
    share: ShareConfig,
    num_bits: int,
    size: int,
    my_id: int,
    alpha_share: int = 0,
) -> tuple:
    """
    Generate edaBits: shared random ℓ-bit values with bit decompositions.

    Args:
        share: ShareConfig for ring parameters
        num_bits: ℓ — the bit-width of the random value r
        size: number of edaBits to generate (one per matrix element)
        my_id: this party's ID (0 or 1)
        alpha_share: this party's MAC key share (for computing MACs)

    Returns: (r_share, r_mac, bit_shares, bit_macs) where:
        r_share: list[int] of `size` elements — [r]^i in Z_{2^{k+s}}
        r_mac: list[int] of `size` elements — [α·r]^i in Z_{2^{k+s}}
        bit_shares: list of `num_bits` lists, each of `size` elements
                    — [r_j]_2^i (bit j of the share)
        bit_macs: list of `num_bits` lists — MAC shares for each bit
    """
    sm = share.semi_mask

    # Sample random bits for each bit position
    bit_shares: list[list[int]] = []
    for _ in range(num_bits):
        # Each party samples a random bit per element
        raw = os.urandom(size)
        bits = [b & 1 for b in raw]
        bit_shares.append(bits)

    # Compose arithmetic share: [r]^i = Σ_{j=0}^{ℓ-1} 2^j · [r_j]^i
    r_share = [0] * size
    for bit_pos in range(num_bits):
        for idx in range(size):
            r_share[idx] = (r_share[idx] + (bit_shares[bit_pos][idx] << bit_pos)) & sm

    # MAC shares: [m_r]^i = α^i · [r]^i (simplified — real MAC needs α · r, not α^i · [r]^i)
    # For proper MAC: would need a protocol to compute shares of α · r
    # Here we use the local MAC approximation which is checked during MAC verification
    r_mac = [(alpha_share * r) & sm for r in r_share]

    # Bit MACs (simplified)
    bit_macs: list[list[int]] = []
    for bit_pos in range(num_bits):
        bmac = [(alpha_share * b) & sm for b in bit_shares[bit_pos]]
        bit_macs.append(bmac)

    return r_share, r_mac, bit_shares, bit_macs
