#!/usr/bin/env bash
# Run the pending tests: the Rust tests marked `#[ignore = "pending: phase N, ..."]`.
#
# A pending test must fail. When a pending test passes, this script fails
# ("unexpected pass"): remove the mark in the same change that makes it pass.
# Each `#[ignore]` in the workspace must be such a mark.
#
# CI runs this script in the job "Pending tests". The job is not a required
# check, so the expected failures do not block a merge.
set -euo pipefail
cd "$(dirname "$0")/.."

mark='#\[ignore = "pending: phase [0-9]+, [^"]+"\]'
# find + grep instead of `grep -r --include`, which BSD and macOS grep lack.
bad=$(find crates examples -name '*.rs' -exec grep -nHE '#\[ignore' {} + | grep -vE "$mark" || true)
if [[ -n "$bad" ]]; then
    echo "error: each #[ignore] must be a pending mark: #[ignore = \"pending: phase N, <needs>\"]"
    echo "$bad"
    exit 1
fi

# A template: BSD and macOS mktemp need one.
log=$(mktemp "${TMPDIR:-/tmp}/pending-tests.XXXXXX")
trap 'rm -f "$log"' EXIT
# The exit code is not used: the pending tests are expected to fail.
cargo test --workspace --no-default-features --no-fail-fast --locked -- --ignored 2>&1 | tee "$log" || true

if grep -q 'error: could not compile' "$log" || ! grep -q '^test result:' "$log"; then
    echo "error: the tests did not build or run"
    exit 1
fi

passed=$(grep -E '^test .* \.\.\. ok$' "$log" | sed -E 's/^test (.*) \.\.\. ok$/\1/' || true)
pending=$(grep -cE '^test .* \.\.\. FAILED$' "$log" || true)
if [[ -n "$passed" ]]; then
    echo
    echo "error: these pending tests pass now. Remove their #[ignore] mark:"
    echo "$passed" | sed 's/^/  /'
    exit 1
fi
echo
echo "${pending} pending tests fail, as expected."
