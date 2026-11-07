#!/usr/bin/env bash
# Check that Cargo.lock takes the dramaturge crates from crates.io, in the
# version that Cargo.toml pins.
#
# A local .cargo/config.toml can patch dramaturge to a working copy, to work
# on both repositories at once. Each cargo command then rewrites Cargo.lock
# with path sources. Such a lockfile must not be committed: CI builds with
# --locked and fails on it, and a build of the committed state would use
# other code than the one tested.
#
# The script checks the staged Cargo.lock (what `git commit` takes), so that
# the local patch may rewrite the working tree while you work. It warns when
# the working tree differs: then `git commit -a` would take the patched file.
set -euo pipefail
cd "$(dirname "$0")/.."

staged=$(git show :Cargo.lock) || {
    echo "error: no Cargo.lock in the git index"
    exit 1
}

python3 - "$staged" <<'PY'
import re, sys, tomllib

lock = tomllib.loads(sys.argv[1])
manifest = tomllib.load(open("Cargo.toml", "rb"))
deps = manifest["workspace"]["dependencies"]
registry = "registry+https://github.com/rust-lang/crates.io-index"
bad = False
for crate in ("dramaturge-model", "dramaturge-runtime", "dramaturge-authoring"):
    want = deps[crate]
    want = want if isinstance(want, str) else want.get("version", "")
    want = want.removeprefix("=")
    found = [p for p in lock.get("package", []) if p["name"] == crate]
    if not found:
        print(f"error: Cargo.lock has no {crate}")
        bad = True
    for p in found:
        source = p.get("source", "a path source")
        if source != registry:
            print(f"error: Cargo.lock does not take {crate} from crates.io ({source})")
            bad = True
        elif p["version"] != want:
            print(f"error: Cargo.lock has {crate} {p['version']}, Cargo.toml pins {want}")
            bad = True
if bad:
    print("Unstage the patched Cargo.lock ('git restore --staged Cargo.lock'), or regenerate it without the local patch.")
    sys.exit(1)
PY

if ! git diff --quiet -- Cargo.lock; then
    echo "note: the Cargo.lock in the working tree differs from the staged one (a local patch?)."
    echo "      Commit with explicit paths, not with 'git commit -a'."
fi
