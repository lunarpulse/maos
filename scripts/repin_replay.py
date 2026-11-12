#!/usr/bin/env python3
"""repin_replay.py -- prove scripts/repin.py against a REAL historical landing commit.

  scripts/repin_replay.py SHA --story 20-13 --adjudicated maos-bin [--note TEXT]

Extracts `SHA~1` into a scratch dir, overlays the landing's source changes
(`git diff SHA~1 SHA -- crates xtask/src`, WITHOUT any pin file), copies the landing's
human-owned surface baseline (docs/ci-baselines/kernel-surface-v0.1-beta.json) in, runs
`repin.py --apply`, and compares the four pin files it wrote with the landing commit's own
after normalising prose (kloc.toml row comments, baseline `#` comments, test `//` comments):
every pin value, hash and digest must match exactly. Also checks `--check` is clean after
the apply and that a second `--apply` is a no-op. Never touches the repo; never runs cargo.
Exit 0 = ALL MATCH.
"""

import argparse
import difflib
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
OWNED = [
    "xtask/kernel-core-baseline.toml",
    "xtask/kloc.toml",
    "xtask/tests/kernel_pin_content_hash_16_0.rs",
    "xtask/tests/recovery_lane_ceiling_rule.rs",
]
SURFACE = "docs/ci-baselines/kernel-surface-v0.1-beta.json"
ROW = re.compile(r'^("[^"]+"|[A-Za-z0-9_.\-]+)[ \t]*=[ \t]*(\d+)[ \t]*(#.*)?$')


def git(*a, **kw):
    return subprocess.run(["git", "-C", str(REPO), *a], capture_output=True, **kw)


def extract(rev, dest: Path, paths=()):
    dest.mkdir(parents=True)
    a = subprocess.Popen(["git", "-C", str(REPO), "archive", rev, *paths], stdout=subprocess.PIPE)
    subprocess.run(["tar", "-x", "-C", str(dest)], stdin=a.stdout, check=True)
    if a.wait() != 0:
        sys.exit(f"git archive {rev} failed")
    for p in dest.rglob("*"):  # fresh mtimes so nothing is treated as cached
        if p.is_file():
            os.utime(p)


def normalise(rel: str, text: str):
    out = []
    for line in text.split("\n"):
        if rel.endswith("kloc.toml"):
            m = ROW.match(line)
            if m:
                out.append(f"{m[1]}={m[2]}")
                continue
        if rel.endswith("baseline.toml") and line.startswith("#"):
            continue
        if rel.endswith(".rs") and line.lstrip().startswith("//"):
            continue
        out.append(line)
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("sha")
    ap.add_argument("--story", required=True)
    ap.add_argument("--adjudicated", default="", help="comma-separated crates the landing overdrew")
    ap.add_argument("--note", default="replay of a historical landing")
    ap.add_argument("--xtask", help="prebuilt xtask (default: $REPIN_XTASK / $CARGO_TARGET_DIR)")
    ap.add_argument("--keep", action="store_true", help="keep the scratch dir")
    args = ap.parse_args()

    scratch = Path(tempfile.mkdtemp(prefix="repin-replay-"))
    try:
        work, base = scratch / "work", scratch / "base"
        extract(f"{args.sha}~1", work)
        extract(f"{args.sha}~1", base, ["crates", "xtask"])
        patch = git("diff", "--binary", f"{args.sha}~1", args.sha, "--", "crates", "xtask/src").stdout
        subprocess.run(["git", "apply", "-"], cwd=work, input=patch, check=True)
        for sub in ("crates", "xtask"):
            for p in (work / sub).rglob("*"):
                if p.is_file():
                    os.utime(p)
        common = [sys.executable, str(HERE / "repin.py"), "--root", str(work), "--base-tree", str(base)]
        if args.xtask:
            common += ["--xtask", args.xtask]

        def run(*a):
            return subprocess.run(common + list(a), capture_output=True, text=True)

        before = run("--check")
        print(f"[1] --check on the overlaid tree: rc {before.returncode} (expected 2 when the landing "
              "overdraws a crate or refreshes the surface)")
        for line in before.stdout.splitlines():
            if line.startswith("  - "):
                print("   ", line[:200])
        # the human steps of the landing: the surface-baseline refresh (never done by repin.py)
        landed_surface = git("show", f"{args.sha}:{SURFACE}", text=True)
        if landed_surface.returncode == 0:
            (work / SURFACE).write_text(landed_surface.stdout, encoding="utf-8")
        apply = ["--apply", "--story", args.story, "--note", args.note]
        for c in filter(None, args.adjudicated.split(",")):
            apply += ["--adjudicated", c]
        done = run(*apply)
        print(f"[2] --apply rc {done.returncode}")
        if done.returncode != 0:
            print(done.stdout[-2500:], done.stderr[-2500:])
            return 1
        again = run(*apply)
        after = run("--check")
        print(f"[3] --check after apply rc {after.returncode}; second --apply rc {again.returncode}: "
              f"{again.stdout.strip().splitlines()[-1] if again.stdout.strip() else ''}")
        ok = after.returncode == 0 and again.returncode == 0
        for rel in OWNED:
            real = git("show", f"{args.sha}:{rel}", text=True).stdout
            mine = (work / rel).read_text(encoding="utf-8")
            diff = list(difflib.unified_diff(normalise(rel, real), normalise(rel, mine),
                                             "landing", "repin", lineterm="", n=0))
            print(f"[4] {rel}: {'MATCH' if not diff else 'DIFF'}")
            if diff:
                ok = False
                print("\n".join(d[:200] for d in diff[:40]))
        print("ALL MATCH" if ok else "MISMATCH")
        return 0 if ok else 1
    finally:
        if args.keep:
            print(f"scratch kept: {scratch}")
        else:
            shutil.rmtree(scratch, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
