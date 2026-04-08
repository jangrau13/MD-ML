#!/bin/bash
set -e

echo "=== MP-SPDZ Party ${PARTY_ID:-0} ==="
echo "API-driven mode: waiting for POST /api/run"

exec python3 bench.py
