#!/usr/bin/env python3
"""repin.py -- re-pin the kernel exact pin per landing; re-base the per-crate ceilings.

Two jobs (SCP 2026-11-12 section 5.B7c, option A):

1. PER LANDING (`--check` / `--apply`): the kernel EXACT pin, after `cargo fmt --all`.

  xtask/kernel-core-baseline.toml        src_lines, [kernel_src] set_hash + per-file
                                         digests, one HISTORY comment block
  xtask/tests/kernel_pin_content_hash_16_0.rs   the four `src_lines` constants

2. AT A RETROSPECTIVE / THE B7c-A LANDING (`--rebase`): the per-crate CEILINGS.

  xtask/kloc.toml                        every crate row := measured + max(100,
                                         ceil(0.02 * measured)); one dated
                                         `# RE-BASED <date> (<reason>)` segment per
                                         changed row. Per-crate rows are ceilings with
                                         stated headroom, NOT per-landing exact pins:
                                         `--check`/`--apply` never touch them. The
                                         zero-headroom `maos-kernel-core` row is HELD
                                         (never raised; `--lower-kernel` lowers it to
                                         its measured value).

It MEASURES with the prebuilt xtask gates (`check-kernel-baseline --json` /
`--emit-pin`, `kloc-check --json`, `check-service-boundary --json`) and never runs
cargo. Python 3, standard library only. See scripts/README-repin.md.

  scripts/repin.py --check                         # table pin vs measured, rc != 0 on drift
  scripts/repin.py --apply --story 20-13 --note "TEXT"   # rewrite the kernel pin, print diff
  scripts/repin.py --rebase --note "REASON" [--measure-root DIR | --measure-json FILE] [--lower-kernel] [--dry-run]

Exit codes: 0 in sync / applied / nothing to do; 1 drift (--check) or a post-write
verification failure (rolled back); 2 a HUMAN decision is needed (nothing written);
3 a gate could not be run / measured.

It NEVER edits docs/ci-baselines/*, docs/invariants, any other file, or reformats a
line it does not own. Every replacement is exactly-once or the run aborts.
"""

from __future__ import annotations

import argparse
import datetime
import difflib
import io
import os
import re
import subprocess
import sys
import tempfile
import textwrap
from pathlib import Path

KERNEL_CRATE = "maos-kernel-core"
BASELINE = "xtask/kernel-core-baseline.toml"
KLOC = "xtask/kloc.toml"
KPIN_TEST = "xtask/tests/kernel_pin_content_hash_16_0.rs"
SURFACE = "docs/ci-baselines/kernel-surface-v0.1-beta.json"
TRACKER = "_bmad-output/implementation-artifacts/sprint-status.yaml"
OWNED_FILES = (BASELINE, KPIN_TEST)

# ADR-069 (see recovery_lane_ceiling_rule.rs): the alarm needs `ENVELOPE ALARM` in the kernel ROW comment.
ADR_069_ALARM = 22_000
ADR_069_ENVELOPE = 23_000
# Mirrors ZERO_HEADROOM_CRATES in xtask/tests/recovery_lane_ceiling_rule.rs: `--rebase` never raises these.
ZERO_HEADROOM_CRATES = (KERNEL_CRATE,)
# ceiling(crate) = measured + max(REBASE_FLOOR, ceil(REBASE_PCT * measured)), PCT as a percentage.
REBASE_FLOOR = 100
REBASE_PCT = 2
LIVE_STATUSES = ("in-progress", "review", "ready-for-dev")
EXIT_OK, EXIT_DRIFT, EXIT_HUMAN, EXIT_TOOL = 0, 1, 2, 3
DIFF_LINE_CAP = 240

ROW_RE = re.compile(
    r'^(?P<key>"[^"]+"|[A-Za-z0-9_.\-]+)(?P<eq>[ \t]*=[ \t]*)(?P<val>\d+)'
    r"(?P<gap>[ \t]*)(?P<comment>#.*)?$"
)
# The segment `--rebase` writes at the head of a row's inline comment (the reason has no `|`).
REBASED_RE = re.compile(r"^RE-BASED \d{4}-\d{2}-\d{2} \([^|]*\)(?: \| |$)")


class Refuse(Exception):
    """A human decision is required; nothing is written."""


class ToolError(Exception):
    """A gate could not be run or its output not understood."""


# ----------------------------------------------------------------------------- gates


def find_xtask(root: Path, explicit: str | None) -> Path:
    cands: list[Path] = []
    if explicit:
        cands.append(Path(explicit))
    if os.environ.get("REPIN_XTASK"):
        cands.append(Path(os.environ["REPIN_XTASK"]))
    td = os.environ.get("CARGO_TARGET_DIR")
    for base in ([Path(td)] if td else []) + [root / "target"]:
        cands += [base / "debug" / "xtask", base / "release" / "xtask"]
    for c in cands:
        if c.is_file() and os.access(c, os.X_OK):
            return c
    raise ToolError(
        "no prebuilt xtask binary found (tried --xtask, $REPIN_XTASK, "
        "$CARGO_TARGET_DIR/{debug,release}/xtask, <root>/target/{debug,release}/xtask); "
        "repin.py never builds -- pass --xtask PATH"
    )


def run_gate(xtask: Path, root: Path, args: list[str]) -> tuple[int, str, str]:
    env = {k: v for k, v in os.environ.items() if k not in ("CI", "GITHUB_ACTIONS")}
    env["CARGO_MANIFEST_DIR"] = str(root / "xtask")
    try:
        p = subprocess.run(
            [str(xtask), *args], cwd=root, env=env, capture_output=True, text=True,
            timeout=600, check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as e:
        raise ToolError(f"xtask {' '.join(args)}: {e}") from e
    return p.returncode, p.stdout, p.stderr


def gate_json(xtask: Path, root: Path, args: list[str]) -> tuple[int, dict]:
    """Run a gate with --json. The gates print their JSON report before returning
    their policy verdict, so a non-zero rc with a JSON body is a report, not a crash."""
    import json

    rc, out, err = run_gate(xtask, root, args)
    try:
        return rc, json.loads(out)
    except ValueError:
        detail = (err.strip() or out.strip())[:600]
        raise ToolError(f"xtask {' '.join(args)} (rc {rc}) produced no JSON report: {detail}")


def parse_emit_pin(block: str) -> tuple[str, dict[str, str]]:
    set_hash = None
    files: dict[str, str] = {}
    in_files = False
    for line in block.splitlines():
        m = re.fullmatch(r'set_hash = "([0-9a-f]{64})"', line)
        if m:
            set_hash = m[1]
        if line == "[kernel_src.files]":
            in_files = True
            continue
        m = re.fullmatch(r'"(.+)" = "([0-9a-f]{64})"', line)
        if in_files and m:
            files[m[1]] = m[2]
    if not set_hash or not files:
        raise ToolError("could not parse `check-kernel-baseline --emit-pin` output")
    return set_hash, files


# ------------------------------------------------------------------------ pin readers


def read(root: Path, rel: str) -> str:
    with open(root / rel, encoding="utf-8", newline="") as f:
        return f.read()


def kloc_rows(text: str) -> dict[str, tuple[int, int]]:
    """key -> (line index, value) for root-level `key = N` rows (before any [table])."""
    rows: dict[str, tuple[int, int]] = {}
    for i, line in enumerate(text.split("\n")):
        if line.startswith("["):
            break
        m = ROW_RE.match(line)
        if m:
            key = m["key"].strip('"')
            if key in rows:
                raise ToolError(f"{KLOC}: duplicate row `{key}` (lines {rows[key][0] + 1}, {i + 1})")
            rows[key] = (i, int(m["val"]))
    return rows


def baseline_pins(text: str) -> dict:
    lines = text.split("\n")
    src_idx = [i for i, l in enumerate(lines) if re.fullmatch(r"src_lines = \d+", l)]
    hash_idx = [i for i, l in enumerate(lines) if re.fullmatch(r'set_hash = "[0-9a-f]{64}"', l)]
    if len(src_idx) != 1 or len(hash_idx) != 1:
        raise ToolError(f"{BASELINE}: expected exactly one `src_lines = N` and one `set_hash = ` line")
    files: dict[str, tuple[int, str]] = {}
    in_files = False
    for i, l in enumerate(lines):
        if l == "[kernel_src.files]":
            in_files = True
            continue
        m = re.fullmatch(r'"(.+)" = "([0-9a-f]{64})"', l)
        if in_files and m:
            files[m[1]] = (i, m[2])
    return {
        "src_idx": src_idx[0],
        "src_lines": int(lines[src_idx[0]].split("=")[1]),
        "hash_idx": hash_idx[0],
        "set_hash": lines[hash_idx[0]].split('"')[1],
        "files": files,
    }


KPIN_PATTERNS = [
    ("read_pinned reader", re.compile(r'(read_pinned\(&baseline_toml\(\)\)\.expect\("read_pinned"\),\s*)(\d[\d_]*)(,)')),
    ("a2a-tcp reader", re.compile(r'(assert_eq!\(a2a, )(\d[\d_]*)(, "the a2a-tcp reader"\);)')),
    ("fkcs oracle reader", re.compile(r'(assert_eq!\(oracle, )(\d[\d_]*)(, "the fkcs oracle reader"\);)')),
    ("toml-parser reader", re.compile(r'(assert_eq!\(parsed\["src_lines"\]\.as_integer\(\), Some\()(\d[\d_]*)(\)\);)')),
]


def kpin_values(text: str) -> dict[str, int]:
    out = {}
    for name, rx in KPIN_PATTERNS:
        found = rx.findall(text)
        if len(found) != 1:
            raise ToolError(f"{KPIN_TEST}: `{name}` constant matched {len(found)} times, expected exactly 1")
        out[name] = int(found[0][1].replace("_", ""))
    return out


def load_tracker(root: Path) -> dict[str, str]:
    path = root / TRACKER
    if not path.is_file():
        return {}
    out: dict[str, str] = {}
    inside = False
    for line in path.read_text(encoding="utf-8").split("\n"):
        if line.startswith("development_status:"):
            inside = True
            continue
        if inside:
            m = re.match(r"^  ([A-Za-z0-9][A-Za-z0-9._\-]*):\s*(\S+)", line)
            if m:
                out[m[1]] = m[2]
            elif line.strip() and not line.startswith(" "):
                break
    return out


def resolve_story(root: Path, story: str) -> str:
    tracker = load_tracker(root)
    if not tracker:
        raise Refuse(f"cannot read the story tracker {TRACKER}")
    if story in tracker:
        key = story
    else:
        hits = [k for k in tracker if k.startswith(story + "-")]
        if len(hits) != 1:
            raise Refuse(
                f"story `{story}` is not a live tracker key"
                + (f" (ambiguous prefix: {', '.join(hits)})" if hits else f" (no key in {TRACKER})")
            )
        key = hits[0]
    if key.startswith("epic-"):
        raise Refuse(f"`{key}` is an epic row; a kernel pin move must cite a live story")
    if tracker[key] not in LIVE_STATUSES:
        raise Refuse(
            f"story `{key}` has tracker status `{tracker[key]}`; a pin move must cite a live story "
            f"({'/'.join(LIVE_STATUSES)}). Ask the operator which open story owns this landing."
        )
    return key


# ------------------------------------------------------------------------- measuring


def measure(root: Path, xtask: Path) -> dict:
    _, kloc = gate_json(xtask, root, ["kloc-check", "--json"])
    _, kb = gate_json(xtask, root, ["check-kernel-baseline", "--json"])
    rc_pin, pin_out, pin_err = run_gate(xtask, root, ["check-kernel-baseline", "--emit-pin"])
    if rc_pin != 0:
        raise ToolError(f"check-kernel-baseline --emit-pin failed: {pin_err.strip()[:400]}")
    set_hash, digests = parse_emit_pin(pin_out)
    if set_hash != kb["set-hash"]:
        raise ToolError("`--emit-pin` set_hash disagrees with `check-kernel-baseline --json`")
    rc_sb, sb = gate_json(xtask, root, ["check-service-boundary", "--json"])
    if KERNEL_CRATE not in kloc["per_crate"]:
        raise ToolError(f"`kloc-check --json` reports no `{KERNEL_CRATE}` measurement")
    return {
        "kernel_tokei": kloc["per_crate"][KERNEL_CRATE],
        "physical": kb["actual-lines"],
        "kb": kb,
        "set_hash": set_hash,
        "digests": digests,
        "sb_passed": bool(sb.get("passed")) and rc_sb == 0,
        "sb_violations": sb.get("violations", []),
    }


# --------------------------------------------------------------------------- analysis


def signed(n: int) -> str:
    return f"{n:+d}"


def row_comment(kloc_text: str, key: str) -> str:
    idx, _ = kloc_rows(kloc_text)[key]
    m = ROW_RE.match(kloc_text.split("\n")[idx])
    return (m["comment"] or "") if m else ""


def analyze(root: Path, m: dict):
    """Returns (table rows, wanted new values, drift items, refusals) for the kernel exact pin."""
    table: list[tuple[str, str, str, str]] = []  # location, pin, measured, status
    drift: list[str] = []
    refusals: list[str] = []
    want: dict = {}

    bp = baseline_pins(read(root, BASELINE))
    kloc_text = read(root, KLOC)
    rows = kloc_rows(kloc_text)
    if KERNEL_CRATE not in rows:
        raise ToolError(f"{KLOC}: no `{KERNEL_CRATE}` row")

    # -- kernel baseline: src_lines, set hash, per-file digests
    st = "ok" if bp["src_lines"] == m["physical"] else "DRIFT"
    table.append((f"{BASELINE}: src_lines", str(bp["src_lines"]), str(m["physical"]), st))
    if st != "ok":
        drift.append("src_lines")
        want["src_lines"] = m["physical"]
    pinned_files = {k: v for k, (_, v) in bp["files"].items()}
    added = sorted(set(m["digests"]) - set(pinned_files))
    removed = sorted(set(pinned_files) - set(m["digests"]))
    changed = sorted(k for k in pinned_files if k in m["digests"] and pinned_files[k] != m["digests"][k])
    table.append((f"{BASELINE}: [kernel_src] files", str(len(pinned_files)), str(len(m["digests"])),
                  "ok" if not (added or removed) else "FILE-SET CHANGED"))
    table.append((f"{BASELINE}: per-file digests", f"{len(changed)} stale", f"{len(changed)} changed",
                  "ok" if not changed else "DRIFT: " + ", ".join(changed)))
    st = "ok" if bp["set_hash"] == m["set_hash"] else "DRIFT"
    table.append((f"{BASELINE}: set_hash", bp["set_hash"][:12] + "..", m["set_hash"][:12] + "..", st))
    if added or removed:
        refusals.append(
            "the kernel file set changed (a new/removed kernel file is a charter decision and moves "
            "PINNED_ENTRIES): " + "; ".join(
                f"{label}: {', '.join(v)}" for label, v in (("added", added), ("removed", removed)) if v)
            + ". Pin by hand with `xtask check-kernel-baseline --emit-pin`.")
    if changed or st != "ok":
        drift.append("digests")
        want["digests_changed"] = changed
        want["set_hash"] = m["set_hash"]
    want["added"], want["removed"] = added, removed

    # -- tests
    kp = kpin_values(read(root, KPIN_TEST))
    for name, val in kp.items():
        st = "ok" if val == m["physical"] else "DRIFT"
        table.append((f"{KPIN_TEST}: {name}", str(val), str(m["physical"]), st))
        if st != "ok":
            want["kpin"] = m["physical"]
            drift.append(f"kpin:{name}")

    # -- kernel ceiling row (read-only here: ceilings move only via --rebase) + ADR-069 envelope
    row = rows[KERNEL_CRATE][1]
    tokei = m["kernel_tokei"]
    if tokei > row:
        table.append((f"{KLOC}: {KERNEL_CRATE} (ceiling)", str(row), str(tokei), f"OVER CEILING +{tokei - row}"))
        refusals.append(
            f"{KERNEL_CRATE} tokei {tokei} is over its kloc.toml ceiling {row} (+{tokei - row}); kernel growth needs "
            "its own measured FLAG-Winston grant on the row, never this command")
    else:
        table.append((f"{KLOC}: {KERNEL_CRATE} (ceiling)", str(row), str(tokei), "ok"))
    if tokei > ADR_069_ENVELOPE:
        refusals.append(f"kernel tokei {tokei} exceeds the ADR-069 envelope {ADR_069_ENVELOPE}: a new operator decision")
    elif tokei > ADR_069_ALARM and "ENVELOPE ALARM" not in row_comment(kloc_text, KERNEL_CRATE):
        refusals.append(
            f"kernel tokei {tokei} crosses the ADR-069 alarm {ADR_069_ALARM}: add `ENVELOPE ALARM` (the "
            f"operator's ack) to the {KERNEL_CRATE} row comment in {KLOC} first (a --note is not enough: the "
            "retained ceiling test reads the row), then re-run")

    # -- surface / service boundary
    if m["sb_passed"]:
        table.append((f"{SURFACE} (check-service-boundary)", "-", "-", "ok"))
    else:
        vs = m["sb_violations"]
        table.append((f"{SURFACE} (check-service-boundary)", "-", "-", f"FAILED ({len(vs)} violations)"))
        syms = sorted({v.get("path", "?") for v in vs})
        refusals.append(
            f"check-service-boundary is red ({len(vs)} violation(s)); a surface-hash change is "
            "operator-granted and the baseline JSON is refreshed by a human, never by this script. Symbols: "
            + ", ".join(syms[:40]) + (" ..." if len(syms) > 40 else ""))
        drift.append("surface")

    want["measured"] = m
    want["bp"] = bp
    return table, want, drift, refusals


def print_table(table):
    w0 = max(len(r[0]) for r in table)
    w1 = max(len(r[1]) for r in table + [("", "PIN", "", "")])
    w2 = max(len(r[2]) for r in table + [("", "", "MEASURED", "")])
    print(f"{'LOCATION'.ljust(w0)}  {'PIN'.rjust(w1)}  {'MEASURED'.rjust(w2)}  STATUS")
    for loc, pin, meas, st in table:
        print(f"{loc.ljust(w0)}  {pin.rjust(w1)}  {meas.rjust(w2)}  {st}")


# ------------------------------------------------------------------------ text edits


def delta_phrase(old: int, new: int) -> str:
    return f"{old} -> {new} ({signed(new - old)})"


def build_texts(root: Path, want: dict, story: str, note: str) -> dict[str, str]:
    m = want["measured"]
    bp = want["bp"]
    texts = {rel: read(root, rel) for rel in OWNED_FILES}
    new_texts: dict[str, str] = {}

    # ---- kernel-core-baseline.toml
    if any(k in want for k in ("src_lines", "set_hash", "digests_changed")):
        blines = texts[BASELINE].split("\n")
        pinned = bp["files"]
        for path in want.get("digests_changed", []):
            i, _ = pinned[path]
            blines[i] = f'"{path}" = "{m["digests"][path]}"'
        if "set_hash" in want:
            blines[bp["hash_idx"]] = f'set_hash = "{want["set_hash"]}"'
        if "src_lines" in want:
            blines[bp["src_idx"]] = f"src_lines = {want['src_lines']}"
        phys = (f"physical {delta_phrase(bp['src_lines'], want['src_lines'])}" if "src_lines" in want
                else f"physical {bp['src_lines']} (unchanged)")
        files = want.get("digests_changed", [])
        text = (f"HISTORY: {story} -> exact formatted {phys}, tokei {m['kernel_tokei']}, {len(m['digests'])} "
                f"files, none added or removed; changed files: {', '.join(files) if files else 'none (set hash only)'}; "
                f"digests and set_hash adopted from `--emit-pin`. {note}")
        block = textwrap.wrap(text, width=96, initial_indent="# ", subsequent_indent="# ",
                              break_long_words=False, break_on_hyphens=False)
        blines[bp["src_idx"] + 1:bp["src_idx"] + 1] = block
        new_texts[BASELINE] = "\n".join(blines)

    # ---- kernel_pin_content_hash_16_0.rs
    if "kpin" in want:
        t = texts[KPIN_TEST]
        for name, rx in KPIN_PATTERNS:
            t, n = rx.subn(lambda mo: f"{mo[1]}{want['kpin']}{mo[3]}", t)
            if n != 1:
                raise ToolError(f"{KPIN_TEST}: `{name}` replaced {n} times, expected exactly 1")
        new_texts[KPIN_TEST] = t
    return new_texts


def validate_new_texts(new_texts: dict[str, str], want: dict):
    """A re-parse of every owned file we wrote."""
    if BASELINE in new_texts:
        bp = baseline_pins(new_texts[BASELINE])
        if "src_lines" in want and bp["src_lines"] != want["src_lines"]:
            raise ToolError("internal: baseline src_lines does not read back")
        if bp["src_idx"] != want["bp"]["src_idx"]:
            raise ToolError("internal: src_lines moved off its line (the 16-0 test pins it)")
        if {k: v for k, (_, v) in bp["files"].items()} != want["measured"]["digests"]:
            raise ToolError("internal: per-file digests do not equal `--emit-pin` after rewrite")
        if bp["set_hash"] != want["measured"]["set_hash"]:
            raise ToolError("internal: set_hash does not equal `--emit-pin` after rewrite")
    if KPIN_TEST in new_texts:
        vals = set(kpin_values(new_texts[KPIN_TEST]).values())
        if vals != {want["kpin"]}:
            raise ToolError("internal: kernel_pin test constants do not all read back the new value")


def print_diff(root: Path, new_texts: dict[str, str]):
    def cap(s: str) -> str:
        return s if len(s) <= DIFF_LINE_CAP else s[:DIFF_LINE_CAP - 20] + f" ...[+{len(s) - DIFF_LINE_CAP + 20} chars]"

    for rel in sorted(new_texts):
        old = read(root, rel).split("\n")
        new = new_texts[rel].split("\n")
        for l in difflib.unified_diff(old, new, f"a/{rel}", f"b/{rel}", lineterm="", n=0):
            print(cap(l))


# ------------------------------------------------------------------------------ rebase


def ceiling_for(measured: int) -> int:
    """measured + max(100, ceil(0.02 * measured)), in integers."""
    return measured + max(REBASE_FLOOR, -(-measured * REBASE_PCT // 100))


def rebase_row(line: str, new_val: int, date: str, reason: str) -> str:
    """The row with its value replaced and ONE `RE-BASED <date> (<reason>)` segment at the
    head of its inline comment (a previous RE-BASED segment is replaced, the rest is kept)."""
    m = ROW_RE.match(line)
    if not m:
        raise ToolError(f"row does not parse: {line[:80]}")
    prior = REBASED_RE.sub("", (m["comment"] or "")[1:].lstrip(), count=1)
    comment = f"# RE-BASED {date} ({reason})" + (f" | {prior}" if prior else "")
    return f"{m['key']}{m['eq']}{new_val}{m['gap'] or '   '}{comment}"


def load_measure_json(path: Path) -> dict[str, int]:
    """The `per_crate` map of a `kloc-check --json` report (any xtask build; a `passed: false` report and
    extra keys are fine). A report without a usable `per_crate` map is refused as a tool error."""
    import json

    try:
        with open(path, encoding="utf-8") as f:
            report = json.load(f)
    except (OSError, ValueError) as e:
        raise ToolError(f"--measure-json {path}: {e}") from e
    per_crate = report.get("per_crate") if isinstance(report, dict) else None
    if not isinstance(per_crate, dict) or not per_crate or not all(
            isinstance(k, str) and isinstance(v, int) and not isinstance(v, bool) and v >= 0
            for k, v in per_crate.items()):
        raise ToolError(f"--measure-json {path}: not a `kloc-check --json` report with a `per_crate` "
                        "map of crate -> tokei lines")
    return per_crate


def rebase_plan(root: Path, measure_root: Path | None, measure_json: Path | None, xtask: Path, date: str,
                reason: str, lower_kernel: bool):
    """Returns (table, new text or None, refusals). The kloc.toml of `root` is the one rewritten;
    the measurement comes from `measure_json` (a saved `kloc-check --json` report) or `measure_root`
    (measured with `xtask`), floored at what `root` measures itself with `xtask`.
    ZERO_HEADROOM_CRATES rows are held (never raised); `lower_kernel` lets them drop to measured."""
    kloc_text = read(root, KLOC)
    rows = kloc_rows(kloc_text)
    if measure_json is not None:
        meas = load_measure_json(measure_json)
    else:
        _, mk = gate_json(xtask, measure_root, ["kloc-check", "--json"])
        meas = mk["per_crate"]
    if measure_json is None and measure_root == root:
        floor = meas
    else:
        _, fk = gate_json(xtask, root, ["kloc-check", "--json"])
        floor = fk["per_crate"]

    refusals: list[str] = []
    missing = sorted(c for c in set(meas) | set(floor) if c not in rows)
    if missing:
        refusals.append("measured crate(s) with no kloc.toml row: " + ", ".join(missing)
                        + " (a new row is a budget decision)")

    lines = kloc_text.split("\n")
    table: list[tuple[str, int, int, int, int, str]] = []  # crate, before, measured, floor, after, status
    for crate, (idx, old) in rows.items():
        if crate.startswith("_"):
            continue  # `_aggregate_alarm` is a warning threshold, not a crate ceiling
        if crate not in meas:
            table.append((crate, old, 0, 0, old, "not measured (kept)"))
            continue
        m, fl = meas[crate], floor.get(crate, 0)
        if m == 0 and fl == 0:
            table.append((crate, old, m, fl, old, "no code (kept)"))  # phantom/tripwire row stays
            continue
        if crate in ZERO_HEADROOM_CRATES:
            held = max(m, fl)  # zero headroom: the only move a re-base may make is down, to measured
            if held > old:
                refusals.append(f"{crate} measures {held}, over its zero-headroom ceiling {old}: it is raised "
                                "only with its own measured grant, never by a re-base")
            if not lower_kernel or held >= old:
                table.append((crate, old, m, fl, old, "held (zero headroom)"))
                continue
            new = held
        else:
            new = max(ceiling_for(m), fl)
        if new == old:
            table.append((crate, old, m, fl, old, "ok"))
            continue
        lines[idx] = rebase_row(lines[idx], new, date, reason)
        table.append((crate, old, m, fl, new, "RE-BASED " + signed(new - old)))

    new_text = "\n".join(lines)
    k = next((r for r in table if r[0] == KERNEL_CRATE), None)
    if k is not None:
        after = k[4]
        if after > ADR_069_ENVELOPE:
            refusals.append(f"{KERNEL_CRATE} row {after} is above the ADR-069 envelope "
                            f"{ADR_069_ENVELOPE}: a new operator decision, not a re-base")
        elif after > ADR_069_ALARM:
            if "ENVELOPE ALARM" not in row_comment(new_text, KERNEL_CRATE):
                refusals.append(f"{KERNEL_CRATE} row {after} is above the ADR-069 alarm {ADR_069_ALARM}: add "
                                f"`ENVELOPE ALARM` (the operator's ack) to the {KERNEL_CRATE} row comment in "
                                f"{KLOC} first, then re-run")
    return table, (new_text if new_text != kloc_text else None), refusals


def print_rebase_table(table):
    hdr = ("CRATE", "BEFORE", "MEASURED", "ROOT-FLOOR", "AFTER", "STATUS")
    body = [(c, str(b), str(m), str(f), str(a), st) for c, b, m, f, a, st in table]
    w = [max(len(r[i]) for r in body + [hdr]) for i in range(5)]
    for r in [hdr] + body:
        print("  ".join(r[i].ljust(w[i]) if i == 0 else r[i].rjust(w[i]) for i in range(5)) + "  " + r[5])


def rebase(root: Path, xtask: Path, args) -> int:
    measure_root = None if args.measure_json else (Path(args.measure_root).resolve() if args.measure_root else root)
    measure_json = Path(args.measure_json).resolve() if args.measure_json else None
    measured_in = measure_json if measure_json else measure_root
    try:
        if measure_root is not None and not (measure_root / KLOC).is_file():
            raise ToolError(f"{measure_root} is not a maos tree: missing {KLOC}")
        if not (root / KLOC).is_file():
            raise ToolError(f"{root} is not a maos repo root: missing {KLOC}")
        reason = " ".join((args.note or "").split())
        if not reason or "|" in reason:
            raise Refuse("--rebase needs --note REASON (the dated RE-BASED segment; no `|` characters)")
        date = args.date or datetime.date.today().isoformat()
        if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", date):
            raise Refuse("--date must be YYYY-MM-DD")
        table, new_text, refusals = rebase_plan(root, measure_root, measure_json, xtask, date, reason,
                                                args.lower_kernel)
    except Refuse as r:
        print("REFUSED -- a human decision is needed; nothing was written:", file=sys.stderr)
        print(f"  - {r}", file=sys.stderr)
        return EXIT_HUMAN
    except ToolError as e:
        print(f"repin: ERROR: {e}", file=sys.stderr)
        return EXIT_TOOL

    print_rebase_table(table)
    changed = sum(1 for r in table if r[5].startswith("RE-BASED"))
    print(f"\n{changed} of {len(table)} crate row(s) change; measured in {measured_in}, "
          f"floored at the measurement of {root}; ceiling = measured + max({REBASE_FLOOR}, ceil({REBASE_PCT}%)); "
          f"zero-headroom rows ({', '.join(ZERO_HEADROOM_CRATES)}) are held"
          + (", lowered to measured (--lower-kernel)" if args.lower_kernel else ""))
    if refusals:
        print("\nREFUSED -- a human decision is needed; nothing was written:", file=sys.stderr)
        for r in refusals:
            print(f"  - {r}", file=sys.stderr)
        return EXIT_HUMAN
    if new_text is None:
        print("\nnothing to do: every crate row already equals its re-based ceiling")
        return EXIT_OK
    if args.dry_run:
        print("\n--dry-run: nothing written")
        return EXIT_OK

    print("\n--- diff ---")
    print_diff(root, {KLOC: new_text})
    original = read(root, KLOC)
    try:
        write_atomic(root / KLOC, new_text)
        after = kloc_rows(read(root, KLOC))
        for crate, _, _, _, new, _ in table:
            if after[crate][1] != new:
                raise ToolError(f"internal: row {crate} reads back {after[crate][1]}, expected {new}")
        rc, k = gate_json(xtask, root, ["kloc-check", "--json"])
        if rc != 0 or not k.get("passed"):
            raise ToolError(f"post-write verification failed: kloc-check passed={k.get('passed')} "
                            f"over_budget={k.get('over_budget')}")
    except (ToolError, OSError) as e:
        write_atomic(root / KLOC, original)
        print(f"repin: ERROR: {e}; {KLOC} restored", file=sys.stderr)
        return EXIT_DRIFT
    print(f"\nAPPLIED to {KLOC}: {changed} row(s) re-based")
    print("verified: kloc-check PASSED; the kernel exact pin is untouched (run `repin.py --check`)")
    return EXIT_OK


# ------------------------------------------------------------------------------ main


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(
        description="Re-pin the kernel exact pin per landing, or re-base the per-crate ceilings, "
                    "from the prebuilt xtask gates (no cargo).")
    mode = ap.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true", help="print kernel pin vs measured; rc != 0 on drift")
    mode.add_argument("--apply", action="store_true", help="rewrite the kernel pin and print the diff")
    mode.add_argument("--rebase", action="store_true",
                      help="re-base every kloc.toml crate row to measured + max(100, ceil(2%%)) "
                           "(retrospectives / the B7c-A landing only); the kernel row is held")
    ap.add_argument("--story", help="live tracker story key (or unique prefix, e.g. 20-13); cited by the baseline HISTORY block")
    ap.add_argument("--note", help="--apply: HISTORY text (driver, ruling); --rebase: the reason (no `|`)")
    src = ap.add_mutually_exclusive_group()
    src.add_argument("--measure-root", metavar="DIR",
                     help="--rebase: tree measured by the prebuilt `xtask kloc-check --json` (default --root; that "
                          "tree's kloc.toml must satisfy the xtask's gate); rows are never lowered below what "
                          "--root itself measures")
    src.add_argument("--measure-json", metavar="FILE",
                     help="--rebase: per-crate measurements read from a `kloc-check --json` report made by any "
                          "xtask build (its `per_crate` map; a passed:false report is fine); --root is still "
                          "measured with --xtask and acts as the floor")
    ap.add_argument("--lower-kernel", action="store_true",
                    help="--rebase: lower the zero-headroom `maos-kernel-core` row to its measured value "
                         "(default off: the kernel row is held, never raised by a re-base)")
    ap.add_argument("--date", help="--rebase: date written into the RE-BASED segment (default today, YYYY-MM-DD)")
    ap.add_argument("--dry-run", action="store_true", help="--rebase: print the table, write nothing")
    ap.add_argument("--root", help="repo root (default: parent of scripts/)")
    ap.add_argument("--xtask", help="prebuilt xtask binary (default: $REPIN_XTASK, $CARGO_TARGET_DIR, <root>/target)")
    args = ap.parse_args(argv)

    root = Path(args.root).resolve() if args.root else Path(__file__).resolve().parent.parent
    if args.measure_json and not args.rebase:
        ap.error("--measure-json is only valid with --rebase")
    if args.rebase:
        try:
            return rebase(root, find_xtask(root, args.xtask), args)
        except ToolError as e:
            print(f"repin: ERROR: {e}", file=sys.stderr)
            return EXIT_TOOL
    try:
        for rel in OWNED_FILES + (KLOC, TRACKER):
            if not (root / rel).is_file():
                raise ToolError(f"{root} is not a maos repo root: missing {rel}")
        xtask = find_xtask(root, args.xtask)
        m = measure(root, xtask)
        table, want, drift, refusals = analyze(root, m)
    except ToolError as e:
        print(f"repin: ERROR: {e}", file=sys.stderr)
        return EXIT_TOOL

    print_table(table)
    print(f"\nmeasured: kernel physical {m['physical']} / tokei {m['kernel_tokei']}, "
          f"{len(m['digests'])} kernel files")

    if args.check:
        if refusals:
            print("\nNEEDS A HUMAN DECISION:")
            for r in refusals:
                print(f"  - {r}")
            return EXIT_HUMAN
        if drift:
            print(f"\nDRIFT in {len(drift)} pin location(s); run `repin.py --apply --story KEY --note TEXT`")
            return EXIT_DRIFT
        print("\nPINS IN SYNC")
        return EXIT_OK

    # ---- apply
    story_key = None
    try:
        if refusals:
            raise Refuse("\n  - ".join(refusals))
        if not drift:
            print("\nnothing to do: every pin already equals the measured value")
            return EXIT_OK
        if not args.story or not args.note or not args.note.strip():
            raise Refuse("--apply needs --story KEY (a live tracker story) and --note TEXT (the HISTORY text) "
                         "because pins would move")
        note = " ".join(args.note.split())
        story_key = resolve_story(root, args.story)
    except Refuse as r:
        print("\nREFUSED -- a human decision is needed; nothing was written:", file=sys.stderr)
        print(f"  - {r}", file=sys.stderr)
        return EXIT_HUMAN

    try:
        new_texts = build_texts(root, want, story_key, note)
        validate_new_texts(new_texts, want)
    except ToolError as e:
        print(f"repin: ERROR: {e}", file=sys.stderr)
        return EXIT_TOOL
    if not new_texts:
        print("\nnothing to do")
        return EXIT_OK

    print(f"\n--- diff (story {story_key}) ---")
    print_diff(root, new_texts)

    originals = {rel: read(root, rel) for rel in new_texts}
    try:
        for rel, text in new_texts.items():
            write_atomic(root / rel, text)
        rc_k, k = gate_json(xtask, root, ["check-kernel-baseline", "--json"])
        if rc_k != 0 or not k.get("passed"):
            raise ToolError(f"post-write verification failed: check-kernel-baseline passed={k.get('passed')}")
    except (ToolError, OSError) as e:
        for rel, text in originals.items():
            write_atomic(root / rel, text)
        print(f"repin: ERROR: {e}; all files restored", file=sys.stderr)
        return EXIT_DRIFT
    print(f"\nAPPLIED to {len(new_texts)} file(s): {', '.join(sorted(new_texts))}")
    print("verified: check-kernel-baseline PASSED; run `repin.py --check` for the full table")
    return EXIT_OK


def write_atomic(path: Path, text: str) -> None:
    mode = path.stat().st_mode
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.repin-")
    try:
        with io.open(fd, "w", encoding="utf-8", newline="") as f:
            f.write(text)
        os.chmod(tmp, mode)
        os.replace(tmp, path)
    except BaseException:
        if os.path.exists(tmp):
            os.unlink(tmp)
        raise


if __name__ == "__main__":
    sys.exit(main())
