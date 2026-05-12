#!/usr/bin/env bash
set -euo pipefail

cargo build --release

benchmarks=(
  "benchmarks/while_sum.imm"
  "benchmarks/for_range_sum.imm"
  "benchmarks/matrix_access.imm"
)

if command -v hyperfine >/dev/null 2>&1; then
  commands=()
  for bench in "${benchmarks[@]}"; do
    commands+=("./target/release/imm-native run ${bench}")
  done
  hyperfine "${commands[@]}"
else
  for bench in "${benchmarks[@]}"; do
    echo "${bench}"
    /usr/bin/time -p ./target/release/imm-native run "${bench}"
  done
fi
