#!/bin/bash
# X Native CI: full test suite + golden project + benchmarks (report-only)
set -e
echo "== workspace tests =="
cargo test --workspace 2>&1 | grep -E "test result:" | awk '{s+=$4; f+=$6} END {print "passed:", s, "failed:", f; exit (f>0)}'
echo "== golden project =="
cargo test -p arco_native --test golden_project 2>&1 | grep "test result"
echo "== benchmarks (informational) =="
cargo build --release -p x-designer --bin bench_scale >/dev/null 2>&1
./target/release/bench_scale
echo "== CI PASS =="
