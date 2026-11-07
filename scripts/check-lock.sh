#!/usr/bin/env bash
# Check that Cargo.lock takes the dramaturge crates from crates.io.
#
# A local .cargo/config.toml can patch dramaturge to a working copy, to work
# on both repositories at once. Each cargo command then rewrites Cargo.lock
# with path sources. Such a lockfile must not be committed: CI builds with
# --locked and fails on it, and a build of the committed state would use
# other code than the one tested.
#
# The script checks the staged Cargo.lock (what a commit takes), so a local
# patch may rewrite the file in the working tree while you work.
set -euo pipefail
cd "$(dirname "$0")/.."

lock=$(git show :Cargo.lock 2>/dev/null || cat Cargo.lock)

bad=0
for crate in dramaturge-model dramaturge-runtime dramaturge-authoring; do
    source=$(awk -v name="$crate" '
        $0 == "name = \"" name "\"" { found = 1; next }
        found && /^source = / { print; exit }
        found && /^\[\[package\]\]/ { exit }
    ' <<<"$lock")
    if [[ "$source" != 'source = "registry+https://github.com/rust-lang/crates.io-index"' ]]; then
        echo "error: Cargo.lock does not take $crate from crates.io (${source:-a path source})"
        bad=1
    fi
done
if [[ $bad -ne 0 ]]; then
    echo "A local patch rewrote Cargo.lock. Unstage it ('git restore --staged Cargo.lock'), or regenerate it without the patch."
    exit 1
fi
