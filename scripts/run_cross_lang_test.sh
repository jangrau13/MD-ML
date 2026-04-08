#!/bin/bash
# Cross-language test runner: C++ party 0 <-> Rust party 1
# Both read preprocessing data generated from the same random values
# (in their respective formats) and communicate over TCP.
#
# Usage: ./scripts/run_cross_lang_test.sh [test_name]
#        If no test_name given, runs all tests.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_DIR/build"
RUST_DIR="$PROJECT_DIR/md-ml-rs"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

TESTS=("multiply_trunc" "multiply" "add" "gtz" "matmul")
PASSED=0
FAILED=0
ERRORS=""

# Expected results for validation
declare -A EXPECTED
EXPECTED[multiply_trunc]="2.250000"
EXPECTED[multiply]="15"
EXPECTED[add]="15"
# gtz: inputs -5,-4,-3,-2,-1,0,1,2,3,4 -> expected 0,0,0,0,0,1,1,1,1,1
# (gtz checks sign bit: 0 is non-negative so gtz(0) = 1)
EXPECTED[gtz]="0,0,0,0,0,1,1,1,1,1"
# matmul: check that both parties agree (we compare C++ vs Rust output)
EXPECTED[matmul]="MATCH"

run_test() {
    local test_name="$1"
    echo -e "${YELLOW}--- Running test: $test_name ---${NC}"

    # Run C++ party 0 and Rust party 1 concurrently
    local cpp_out=$(mktemp)
    local rs_out=$(mktemp)
    local cpp_err=$(mktemp)
    local rs_err=$(mktemp)

    # Start Rust party 1 first (it listens for connections)
    cd "$RUST_DIR"
    cargo run --release --bin cross_lang_party_1 -- "$test_name" \
        >"$rs_out" 2>"$rs_err" &
    local rs_pid=$!

    # Small delay to let Rust party start listening
    sleep 0.5

    # Start C++ party 0
    "$BUILD_DIR/experiments/cross-lang-test/cross_lang_party_0" "$test_name" \
        >"$cpp_out" 2>"$cpp_err" &
    local cpp_pid=$!

    # Wait for both with timeout
    local timeout=60
    local ok=true

    if ! wait_with_timeout $cpp_pid $timeout; then
        echo -e "${RED}  C++ party 0 timed out or failed${NC}"
        cat "$cpp_err" >&2
        kill $rs_pid 2>/dev/null || true
        ok=false
    fi

    if ! wait_with_timeout $rs_pid $timeout; then
        echo -e "${RED}  Rust party 1 timed out or failed${NC}"
        cat "$rs_err" >&2
        ok=false
    fi

    if [ "$ok" = false ]; then
        FAILED=$((FAILED + 1))
        ERRORS="$ERRORS  FAIL: $test_name (process error)\n"
        rm -f "$cpp_out" "$rs_out" "$cpp_err" "$rs_err"
        return
    fi

    # Extract RESULT lines
    local cpp_result=$(grep "^RESULT:" "$cpp_out" | head -1 | sed 's/^RESULT://')
    local rs_result=$(grep "^RESULT:" "$rs_out" | head -1 | sed 's/^RESULT://')

    echo "  C++ output: $cpp_result"
    echo "  Rust output: $rs_result"

    # Check that both parties agree
    if [ "$cpp_result" != "$rs_result" ]; then
        echo -e "${RED}  MISMATCH: C++ and Rust produced different results!${NC}"
        FAILED=$((FAILED + 1))
        ERRORS="$ERRORS  FAIL: $test_name (C++='$cpp_result' vs Rust='$rs_result')\n"
        rm -f "$cpp_out" "$rs_out" "$cpp_err" "$rs_err"
        return
    fi

    # Check against expected value (if not matmul)
    local expected="${EXPECTED[$test_name]}"
    if [ "$expected" = "MATCH" ]; then
        # For matmul, just check that both agree
        echo -e "${GREEN}  PASS: Both parties agree${NC}"
        PASSED=$((PASSED + 1))
    elif [ "$cpp_result" = "$expected" ]; then
        echo -e "${GREEN}  PASS: Result matches expected ($expected)${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}  FAIL: Expected '$expected' but got '$cpp_result'${NC}"
        FAILED=$((FAILED + 1))
        ERRORS="$ERRORS  FAIL: $test_name (expected='$expected' got='$cpp_result')\n"
    fi

    rm -f "$cpp_out" "$rs_out" "$cpp_err" "$rs_err"
}

wait_with_timeout() {
    local pid=$1
    local timeout=$2
    local count=0
    while kill -0 "$pid" 2>/dev/null; do
        sleep 1
        count=$((count + 1))
        if [ $count -ge $timeout ]; then
            kill "$pid" 2>/dev/null || true
            return 1
        fi
    done
    wait "$pid"
    return $?
}

# ---- Build phase ----
echo -e "${YELLOW}=== Building C++ ===${NC}"
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"
cmake .. -DCMAKE_BUILD_TYPE=Release 2>&1 | tail -3
cmake --build . --target cross_lang_party_0 -j$(nproc 2>/dev/null || sysctl -n hw.ncpu) 2>&1 | tail -5

echo -e "${YELLOW}=== Building Rust ===${NC}"
cd "$RUST_DIR"
cargo build --release --bin cross_lang_fake_offline --bin cross_lang_party_1 2>&1 | tail -5

# ---- Generate preprocessing data ----
echo -e "${YELLOW}=== Generating preprocessing data ===${NC}"
cargo run --release --bin cross_lang_fake_offline 2>&1

# ---- Run tests ----
echo ""
echo -e "${YELLOW}=== Running cross-language tests ===${NC}"
echo ""

if [ $# -gt 0 ]; then
    # Run specific test
    run_test "$1"
else
    # Run all tests
    for test in "${TESTS[@]}"; do
        run_test "$test"
        echo ""
    done
fi

# ---- Summary ----
echo -e "${YELLOW}=== Summary ===${NC}"
echo -e "${GREEN}Passed: $PASSED${NC}"
if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Failed: $FAILED${NC}"
    echo -e "${RED}Failures:${NC}"
    echo -e "$ERRORS"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
fi
