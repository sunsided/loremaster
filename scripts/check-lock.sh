#!/usr/bin/env bash
# Check that Cargo.lock takes the dramaturge crates from crates.io, in a
# version that the requirement in Cargo.toml allows.
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


def allows(req, version):
    """Cargo's rule for `=x.y.z[-pre]` (exact) and a plain `x[.y[.z]]`
    (caret). Other forms raise `ValueError`."""
    version = version.split("+")[0]  # build metadata does not count
    # `=x.y.z[-pre]` (exact) or `x[.y[.z]]` (caret); other forms raise.
    pre = r"[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*"  # dot-separated, no empty part
    if not re.fullmatch(rf"=\d+\.\d+\.\d+(-{pre})?|\d+(\.\d+){{0,2}}", req):
        raise ValueError(req)
    if req.startswith("="):
        return version == req[1:]
    if "-" in version:
        return False  # a plain requirement does not take pre-releases
    want = [int(x) for x in req.split(".")]
    have = [int(x) for x in version.split(".")]
    # The leftmost non-zero part must match: 0.3 takes 0.3.x, 1.2 takes 1.x.
    first = next((i for i, x in enumerate(want) if x != 0), len(want) - 1)
    return have[: first + 1] == want[: first + 1] and have[: len(want)] >= want


assert allows("0.3", "0.3.1") and allows("0.3", "0.3.0") and not allows("0.3", "0.4.0")
assert not allows("0.3", "0.3.0-alpha.5") and allows("=0.3.0-alpha.5", "0.3.0-alpha.5")
assert allows("1.2", "1.9.0") and not allows("1.2", "1.1.9") and not allows("1.2", "2.0.0")
assert allows("0.3", "0.3.1+build")
assert allows("=0.4.0-alpha-1", "0.4.0-alpha-1")
for unsupported in ("=0.3", "0.3.0-alpha.5", ">=0.3.0", "~0.3", "*", "", "=0.3.0-", "=0.3.0-..", "=0.3.0-.5"):
    try:
        allows(unsupported, "0.3.0")
    except ValueError:
        continue
    raise AssertionError(f"accepted {unsupported!r}")

bad = False
for crate in (
    "dramaturge-model",
    "dramaturge-runtime",
    "dramaturge-authoring",
    "dramaturge-syntax",
):
    want = deps[crate]
    want = want if isinstance(want, str) else want.get("version", "")
    found = [p for p in lock.get("package", []) if p["name"] == crate]
    if not found:
        print(f"error: Cargo.lock has no {crate}")
        bad = True
    for p in found:
        source = p.get("source", "a path source")
        if source != registry:
            print(f"error: Cargo.lock does not take {crate} from crates.io ({source})")
            bad = True
            continue
        try:
            ok = allows(want, p["version"])
        except ValueError:
            print(f"error: check-lock.sh only reads `=x.y.z[-pre]` and `x[.y[.z]]`; {crate} asks for {want!r}")
            bad = True
            continue
        if not ok:
            print(f"error: Cargo.lock has {crate} {p['version']}, Cargo.toml asks for {want}")
            bad = True
if bad:
    print("Unstage the patched Cargo.lock ('git restore --staged Cargo.lock'), or regenerate it without the local patch.")
    sys.exit(1)
PY

if ! git diff --quiet -- Cargo.lock; then
    echo "note: the Cargo.lock in the working tree differs from the staged one (a local patch?)."
    echo "      Commit with explicit paths, not with 'git commit -a'."
fi
