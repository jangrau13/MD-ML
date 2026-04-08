"""
π_MACCheck — Batch MAC Verification (Appendix B.1).

Verifies that opened values are consistent with their SPDZ-2k MACs.
For each opened value x with MAC m_x = α · x mod 2^{k+s}:
  party i holds [m_x]^i (MAC share) and α^i (key share).

Verification uses a random linear combination:
  σ^i = Σ_j r_j · ([m_j]^i − α^i · x_j)
Then parties check that Σ σ^i = 0 (over Z_{2^{k+s}}).
"""

from __future__ import annotations
import hashlib
from ..share import ShareConfig


def mac_check(
    share: ShareConfig,
    opened_values: list[int],
    mac_shares: list[int],
    alpha_share: int,
    net,
    my_id: int,
    other_id: int,
) -> bool:
    """
    Verify MACs on a batch of opened values.

    Args:
        share: ShareConfig for the ring parameters
        opened_values: list of opened cleartext values x_j
        mac_shares: list of this party's MAC shares [m_j]^i
        alpha_share: this party's MAC key share α^i
        net: Party networking object with send_recv_concurrent()
        my_id: this party's ID
        other_id: the other party's ID

    Returns: True if MAC check passes, False otherwise.
    """
    sm = share.semi_mask
    n = len(opened_values)
    if n == 0:
        return True

    # Derive random coefficients from opened values (public coin)
    # In a real implementation, use a proper coin-tossing protocol
    seed = b"mac_check_" + b"".join(
        v.to_bytes(share.semi_bytes, "little") for v in opened_values[:min(4, n)]
    )
    coeffs = _derive_coefficients(seed, n, sm, share.semi_bytes)

    # σ^i = Σ_j r_j · ([m_j]^i − α^i · x_j) mod 2^{k+s}
    sigma = 0
    for j in range(n):
        diff = (mac_shares[j] - alpha_share * opened_values[j]) & sm
        sigma = (sigma + coeffs[j] * diff) & sm

    # Exchange σ shares and verify sum = 0
    send_data = sigma.to_bytes(share.semi_bytes, "little")
    recv_data = net.send_recv_concurrent(other_id, send_data, share.semi_bytes)
    other_sigma = int.from_bytes(recv_data, "little")

    total = (sigma + other_sigma) & sm
    return total == 0


def _derive_coefficients(seed: bytes, n: int, mask: int, byte_width: int) -> list[int]:
    """Derive n pseudorandom coefficients from a seed."""
    coeffs = []
    for i in range(n):
        h = hashlib.sha256(seed + i.to_bytes(4, "little")).digest()
        coeffs.append(int.from_bytes(h[:byte_width], "little") & mask)
    return coeffs
