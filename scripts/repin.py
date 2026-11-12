#!/usr/bin/env python3
"""repin.py -- re-pin the kernel / per-crate line-count pins in ONE command.

Replaces the manual pin set every landing edits by hand after `cargo fmt --all`:

  xtask/kernel-core-baseline.toml        src_lines, [kernel_src] set_hash + per-file
                                         digests, one HISTORY comment block
  xtask/kloc.toml                        per-crate rows, `_aggregate_hardfail`
                                         (= measured aggregate + 1), HISTORY segments
  xtask/tests/kernel_pin_content_hash_16_0.rs   the four `src_lines` constants
  xtask/tests/recovery_lane_ceiling_rule.rs     RATIFIED_AT_EASING

It MEASURES with the prebuilt xtask gates (`check-kernel-baseline --json` /
`--emit-pin`, `kloc-check --json`, `check-service-boundary --json`) and never runs
cargo. Python 3, standard library only. See scripts/README-repin.md.

  scripts/repin.py --check                         # table pin vs measured, rc != 0 on drift
  scripts/repin.py --apply --story 20-13 --note "TEXT"   # rewrite pins, print diff

Exit codes: 0 in sync / applied / nothing to do; 1 drift (--check) or a post-write
verification failure (rolled back); 2 a HUMAN decision is needed (nothing written);
3 a gate could not be run / measured.

It NEVER edits docs/ci-baselines/*, docs/invariants, any other file, or reformats a
line it does not own. Every replacement is exactly-once or the run aborts.
"""

from __future__ import annotations

import argparse
import difflib
import io
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import textwrap
from pathlib import Path

KERNEL_CRATE = "maos-kernel-core"
BASELINE = "xtask/kernel-core-baseline.toml"
KLOC = "xtask/kloc.toml"
KPIN_TEST = "xtask/tests/kernel_pin_content_hash_16_0.rs"
RECOVERY_TEST = "xtask/tests/recovery_lane_ceiling_rule.rs"
SURFACE = "docs/ci-baselines/kernel-surface-v0.1-beta.json"
TRACKER = "_bmad-output/implementation-artifacts/sprint-status.yaml"
OWNED_FILES = (BASELINE, KLOC, KPIN_TEST, RECOVERY_TEST)

# ADR-069 (see recovery_lane_ceiling_rule.rs): alarm needs `ENVELOPE ALARM` in the row.
ADR_069_ALARM = 22_000
ADR_069_ENVELOPE = 23_000
LIVE_STATUSES = ("in-progress", "review", "ready-for-dev")
EXIT_OK, EXIT_DRIFT, EXIT_HUMAN, EXIT_TOOL = 0, 1, 2, 3
DIFF_LINE_CAP = 240

ROW_RE = re.compile(
    r'^(?P<key>"[^"]+"|[A-Za-z0-9_.\-]+)(?P<eq>[ \t]*=[ \t]*)(?P<val>\d+)'
    r"(?P<gap>[ \t]*)(?P<comment>#.*)?$"
)
ARROW_RE = re.compile(r"\b([0-9]+)\s*(?:→|->)\s*([0-9]+)\b")  # == d11 ratchet's arrow_grants


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
RECOVERY_RE = re.compile(r"^(?P<pre>[ \t]*const RATIFIED_AT_EASING: i64 = )(?P<val>\d[\d_]*)(?P<post>;)$", re.M)


def kpin_values(text: str) -> dict[str, int]:
    out = {}
    for name, rx in KPIN_PATTERNS:
        found = rx.findall(text)
        if len(found) != 1:
            raise ToolError(f"{KPIN_TEST}: `{name}` constant matched {len(found)} times, expected exactly 1")
        out[name] = int(found[0][1].replace("_", ""))
    return out


def recovery_value(text: str) -> int:
    found = RECOVERY_RE.findall(text)
    if len(found) != 1:
        raise ToolError(f"{RECOVERY_TEST}: RATIFIED_AT_EASING matched {len(found)} times, expected exactly 1")
    return int(found[0][1].replace("_", ""))


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
    if key not in story_keys(key):
        raise Refuse(f"`{key}` is not a story key (epic rows cannot own a pin move; the d11 ratchet refuses them)")
    if tracker[key] not in LIVE_STATUSES:
        raise Refuse(
            f"story `{key}` has tracker status `{tracker[key]}`; a pin move must cite a live story "
            f"({'/'.join(LIVE_STATUSES)}). Ask the operator which open story owns this landing."
        )
    return key


# ------------------------------------------------------------------------- measuring


def measure(root: Path, xtask: Path) -> dict:
    rc, kloc = gate_json(xtask, root, ["kloc-check", "--json"])
    rc_kb, kb = gate_json(xtask, root, ["check-kernel-baseline", "--json"])
    rc_pin, pin_out, pin_err = run_gate(xtask, root, ["check-kernel-baseline", "--emit-pin"])
    if rc_pin != 0:
        raise ToolError(f"check-kernel-baseline --emit-pin failed: {pin_err.strip()[:400]}")
    set_hash, digests = parse_emit_pin(pin_out)
    if set_hash != kb["set-hash"]:
        raise ToolError("`--emit-pin` set_hash disagrees with `check-kernel-baseline --json`")
    rc_sb, sb = gate_json(xtask, root, ["check-service-boundary", "--json"])
    return {
        "per_crate": kloc["per_crate"],
        "aggregate": kloc["aggregate"],
        "physical": kb["actual-lines"],
        "kb": kb,
        "set_hash": set_hash,
        "digests": digests,
        "sb_passed": bool(sb.get("passed")) and rc_sb == 0,
        "sb_violations": sb.get("violations", []),
    }


def measure_base(root: Path, xtask: Path, base_ref: str | None, base_tree: str | None):
    """Per-crate (pin, measured) of the PRE-landing tree: tells exact-pinned rows (pin ==
    measured, lowered with the code) from reserve rows (pin > measured, left alone)."""
    tmp = None
    try:
        if base_tree:
            tree = Path(base_tree)
        else:
            if subprocess.run(["git", "-C", str(root), "rev-parse", "--git-dir"],
                              capture_output=True).returncode != 0:
                return None, "no git checkout and no --base-tree"
            tmp = tempfile.mkdtemp(prefix="repin-base-")
            p = subprocess.Popen(
                ["git", "-C", str(root), "archive", base_ref or "HEAD", "crates", "xtask"],
                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            with tarfile.open(fileobj=p.stdout, mode="r|") as tf:
                try:
                    tf.extractall(tmp, filter="data")
                except TypeError:  # Python < 3.12
                    tf.extractall(tmp)
            if p.wait() != 0:
                return None, f"git archive {base_ref or 'HEAD'} failed: {p.stderr.read().decode()[:200]}"
            tree = Path(tmp)
        _, kloc = gate_json(xtask, tree, ["kloc-check", "--json"])
        rows = kloc_rows(read(tree, KLOC))
        return {c: (rows[c][1] if c in rows else None, m) for c, m in kloc["per_crate"].items()}, ""
    except (ToolError, OSError, tarfile.TarError) as e:
        return None, str(e)
    finally:
        if tmp:
            shutil.rmtree(tmp, ignore_errors=True)


# --------------------------------------------------------------------------- analysis


def signed(n: int) -> str:
    return f"{n:+d}"


def analyze(root: Path, m: dict, base, adjudicated: set[str], lower: set[str], note: str | None):
    """Returns (table rows, wanted new values, drift items, refusals)."""
    table: list[tuple[str, str, str, str]] = []  # location, pin, measured, status
    drift: list[str] = []
    refusals: list[str] = []
    want: dict = {"rows": {}, "row_kind": {}}

    base_text = read(root, BASELINE)
    bp = baseline_pins(base_text)
    kloc_text = read(root, KLOC)
    rows = kloc_rows(kloc_text)

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

    # -- per-crate rows
    per = m["per_crate"]
    missing = sorted(c for c in per if c not in rows)
    if missing:
        refusals.append("measured crate(s) with no kloc.toml row: " + ", ".join(missing)
                        + " (a new row is a budget decision)")
    for crate, meas in sorted(per.items()):
        if crate not in rows:
            continue
        pin = rows[crate][1]
        base_pin, base_meas = (base.get(crate, (None, None)) if base else (None, None))
        exact_row = (crate == KERNEL_CRATE) or crate in lower or (
            base is not None and base_pin is not None and base_pin == base_meas)
        if meas == pin:
            table.append((f"{KLOC}: {crate}", str(pin), str(meas), "ok"))
            continue
        if meas > pin:
            if crate in adjudicated:
                status, kind = f"OVERDRAFT (adjudicated, +{meas - pin})", "overdraft"
                want["rows"][crate] = meas
                want["row_kind"][crate] = kind
                drift.append(crate)
            else:
                status = f"OVERDRAFT +{meas - pin}"
                refusals.append(
                    f"{crate} is over its ceiling: kloc.toml row {pin}, measured {meas} "
                    f"(+{meas - pin})"
                    + (f", pre-landing measured {base_meas}" if base_meas is not None else "")
                    + (f"; kernel physical {m['physical']}" if crate == KERNEL_CRATE else "")
                    + ". The orchestrator adjudicates; then re-run with `--adjudicated " + crate + "`.")
        elif exact_row:
            status = f"LOWER {signed(meas - pin)}"
            want["rows"][crate] = meas
            want["row_kind"][crate] = "lower"
            drift.append(crate)
        else:
            status = "reserve row (kept)"
        table.append((f"{KLOC}: {crate}", str(pin), str(meas), status))
    for crate in sorted(adjudicated - set(want["rows"]) - {c for c, v in per.items() if c in rows and v <= rows[c][1]}):
        refusals.append(f"--adjudicated {crate}: unknown crate or not measured (typo?)")

    # -- aggregate
    if "_aggregate_hardfail" not in rows:
        raise ToolError(f"{KLOC}: no `_aggregate_hardfail` row")
    hard_pin = rows["_aggregate_hardfail"][1]
    want_hard = m["aggregate"] + 1
    st = "ok" if hard_pin == want_hard else "DRIFT"
    table.append((f"{KLOC}: _aggregate_hardfail", str(hard_pin), f"{m['aggregate']}+1={want_hard}", st))
    if st != "ok":
        drift.append("_aggregate_hardfail")
        want["hardfail"] = want_hard

    # -- tests
    kp = kpin_values(read(root, KPIN_TEST))
    for name, val in kp.items():
        st = "ok" if val == m["physical"] else "DRIFT"
        table.append((f"{KPIN_TEST}: {name}", str(val), str(m["physical"]), st))
        if st != "ok":
            want["kpin"] = m["physical"]
            drift.append(f"kpin:{name}")
    rec = recovery_value(read(root, RECOVERY_TEST))
    new_kernel_row = want["rows"].get(KERNEL_CRATE, rows[KERNEL_CRATE][1])
    st = "ok" if rec == new_kernel_row else "DRIFT"
    table.append((f"{RECOVERY_TEST}: RATIFIED_AT_EASING", str(rec), str(new_kernel_row), st))
    if st != "ok":
        want["recovery"] = new_kernel_row
        drift.append("RATIFIED_AT_EASING")

    # -- kernel ADR-069 envelope
    if KERNEL_CRATE in want["rows"]:
        k = want["rows"][KERNEL_CRATE]
        if k > ADR_069_ENVELOPE:
            refusals.append(f"kernel tokei {k} exceeds the ADR-069 envelope {ADR_069_ENVELOPE}: a new operator decision")
        elif k > ADR_069_ALARM and "ENVELOPE ALARM" not in (note or ""):
            refusals.append(
                f"kernel tokei {k} crosses the ADR-069 alarm {ADR_069_ALARM}: the row HISTORY must carry "
                "`ENVELOPE ALARM` (pass it in --note) after the operator ack")

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
    want["rows_state"] = rows
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


def story_keys(segment: str) -> list[str]:
    """Port of d11_xtask_ceiling_ratchet.rs `story_keys`."""
    out = []
    for token in re.split(r"[^A-Za-z0-9\-]", segment):
        token = token.strip("-")
        parts = token.split("-")
        if len(parts) >= 3 and parts[0] and parts[0].isdigit() and parts[1][:1].isdigit():
            out.append(token)
    return out


def rewrite_row(line: str, new_val: int, segment: str) -> str:
    m = ROW_RE.match(line)
    if not m:
        raise ToolError(f"row does not parse: {line[:80]}")
    old_comment = (m["comment"] or "")[1:].lstrip()
    comment = f"# {segment}" + (f" | PRIOR: {old_comment}" if old_comment else "")
    gap = m["gap"] or "   "
    return f"{m['key']}{m['eq']}{new_val}{gap}{comment}"


def delta_phrase(old: int, new: int) -> str:
    return f"{old} -> {new} ({signed(new - old)})"


def build_texts(root: Path, want: dict, story: str, note: str, base) -> dict[str, str]:
    m = want["measured"]
    rows = want["rows_state"]
    bp = want["bp"]
    texts = {rel: read(root, rel) for rel in OWNED_FILES}
    new_texts: dict[str, str] = {}

    # ---- kloc.toml
    lines = texts[KLOC].split("\n")
    stamp = "exact formatted cargo fmt --all / kloc-check --json"
    for crate, new_val in want["rows"].items():
        idx, old_val = rows[crate]
        extra = ""
        if crate == KERNEL_CRATE and "src_lines" in want:
            extra = f"; physical {delta_phrase(bp['src_lines'], want['src_lines'])}"
        if want["row_kind"][crate] == "overdraft":
            extra += f"; overdraft over the live pin {old_val}, adjudicated"
        elif want["row_kind"][crate] == "lower":
            extra += "; lower re-pin"
        seg = f"HISTORY: {story} -> {stamp}: {delta_phrase(old_val, new_val)}{extra}; {note}"
        lines[idx] = rewrite_row(lines[idx], new_val, seg)
    if "hardfail" in want:
        idx, old_val = rows["_aggregate_hardfail"]
        parts = []
        for crate in sorted(m["per_crate"]):
            if crate.startswith("("):  # fallback labels (spikes/templates): not in the partial base tree
                continue
            if base and crate in base and base[crate][1] is not None and m["per_crate"][crate] != base[crate][1]:
                parts.append(f"{crate.replace('maos-', '')} {signed(m['per_crate'][crate] - base[crate][1])}")
            elif not base and crate in want["rows"]:
                parts.append(f"{crate.replace('maos-', '')} {signed(want['rows'][crate] - rows[crate][1])}")
        seg = (f"HISTORY: {story} -> {stamp}: {old_val} -> {want['hardfail']} (measured {m['aggregate']} + 1, "
               f"{signed(want['hardfail'] - old_val)}" + (f"; {', '.join(parts)}" if parts else "") + f"); {note}")
        lines[idx] = rewrite_row(lines[idx], want["hardfail"], seg)
    if lines != texts[KLOC].split("\n"):
        new_texts[KLOC] = "\n".join(lines)

    # ---- kernel-core-baseline.toml
    if any(k in want for k in ("src_lines", "set_hash", "digests_changed")):
        blines = texts[BASELINE].split("\n")
        pinned = want["bp"]["files"]
        for path in want.get("digests_changed", []):
            i, _ = pinned[path]
            blines[i] = f'"{path}" = "{m["digests"][path]}"'
        if "set_hash" in want:
            blines[bp["hash_idx"]] = f'set_hash = "{want["set_hash"]}"'
        if "src_lines" in want:
            blines[bp["src_idx"]] = f"src_lines = {want['src_lines']}"
        phys = (f"physical {delta_phrase(bp['src_lines'], want['src_lines'])}" if "src_lines" in want
                else f"physical {bp['src_lines']} (unchanged)")
        k_old = rows[KERNEL_CRATE][1]
        k_new = want["rows"].get(KERNEL_CRATE, k_old)
        tok = f"tokei {delta_phrase(k_old, k_new)}" if k_new != k_old else f"tokei {k_old} (unchanged)"
        files = want.get("digests_changed", [])
        text = (f"HISTORY: {story} -> exact formatted {phys}, {tok}, {len(m['digests'])} files, none added "
                f"or removed; changed files: {', '.join(files) if files else 'none (set hash only)'}; digests "
                f"and set_hash adopted from `--emit-pin`. {note}")
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

    # ---- recovery_lane_ceiling_rule.rs
    if "recovery" in want:
        t = texts[RECOVERY_TEST]
        k_old = rows[KERNEL_CRATE][1]
        phys = f", physical {delta_phrase(bp['src_lines'], want['src_lines'])}" if "src_lines" in want else ""
        ledger = textwrap.wrap(
            f"{story}: exact formatted {delta_phrase(k_old, want['recovery'])}{phys}; {note}",
            width=88, initial_indent="    // ", subsequent_indent="    // ",
            break_long_words=False, break_on_hyphens=False)
        t, n = RECOVERY_RE.subn(
            lambda mo: f"{mo['pre']}{want['recovery']:_}{mo['post']}\n" + "\n".join(ledger), t)
        if n != 1:
            raise ToolError(f"{RECOVERY_TEST}: RATIFIED_AT_EASING replaced {n} times, expected exactly 1")
        new_texts[RECOVERY_TEST] = t
    return new_texts


def validate_new_texts(root: Path, new_texts: dict[str, str], want: dict, story: str):
    """The d11 contract for every row we wrote: first segment cites the live story and its
    first arrow is old -> new; plus a re-parse of every owned file."""
    tracker = load_tracker(root)
    if KLOC in new_texts:
        after = kloc_rows(new_texts[KLOC])
        lines = new_texts[KLOC].split("\n")
        written = dict(want["rows"])
        if "hardfail" in want:
            written["_aggregate_hardfail"] = want["hardfail"]
        for key, new_val in written.items():
            idx, val = after[key]
            if val != new_val:
                raise ToolError(f"internal: row {key} reads back {val}, expected {new_val}")
            comment = lines[idx].split("#", 1)[1]
            first = comment.split("|")[0]
            if story not in story_keys(first) or tracker.get(story) not in LIVE_STATUSES:
                raise ToolError(f"internal: row {key}: first HISTORY segment does not cite live story {story}")
            arrows = ARROW_RE.findall(first)
            if not arrows or int(arrows[0][1]) != new_val:
                raise ToolError(f"internal: row {key}: first arrow of the HISTORY segment is not `old -> {new_val}`")
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
    if RECOVERY_TEST in new_texts and recovery_value(new_texts[RECOVERY_TEST]) != want["recovery"]:
        raise ToolError("internal: RATIFIED_AT_EASING does not read back")


def print_diff(root: Path, new_texts: dict[str, str]):
    def cap(s: str) -> str:
        return s if len(s) <= DIFF_LINE_CAP else s[:DIFF_LINE_CAP - 20] + f" ...[+{len(s) - DIFF_LINE_CAP + 20} chars]"

    for rel in OWNED_FILES:
        if rel not in new_texts:
            continue
        old = read(root, rel).split("\n")
        new = new_texts[rel].split("\n")
        for l in difflib.unified_diff(old, new, f"a/{rel}", f"b/{rel}", lineterm="", n=0):
            print(cap(l))


# ------------------------------------------------------------------------------ main


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(
        description="Re-pin the kernel/crate line-count pins from the prebuilt xtask gates (no cargo).")
    mode = ap.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true", help="print pin vs measured; rc != 0 on drift")
    mode.add_argument("--apply", action="store_true", help="rewrite the pins and print the diff")
    ap.add_argument("--story", help="live tracker story key (or unique prefix, e.g. 20-13); cited by the HISTORY segments")
    ap.add_argument("--note", help="HISTORY text (driver, ruling); no `|` characters")
    ap.add_argument("--adjudicated", action="append", default=[], metavar="CRATE",
                    help="the orchestrator adjudicated this crate's overdraft: pin it to the measured value (repeatable)")
    ap.add_argument("--lower", action="append", default=[], metavar="CRATE",
                    help="force a lower re-pin of a reserve row (rows are lowered automatically only when "
                         "they were exact before the landing)")
    ap.add_argument("--root", help="repo root (default: parent of scripts/)")
    ap.add_argument("--xtask", help="prebuilt xtask binary (default: $REPIN_XTASK, $CARGO_TARGET_DIR, <root>/target)")
    ap.add_argument("--base", default="HEAD", help="git ref of the pre-landing tree (default HEAD)")
    ap.add_argument("--base-tree", help="directory holding the pre-landing crates/ and xtask/ (instead of --base)")
    args = ap.parse_args(argv)

    root = Path(args.root).resolve() if args.root else Path(__file__).resolve().parent.parent
    adjudicated = {c for a in args.adjudicated for c in a.split(",") if c}
    lower = {c for a in args.lower for c in a.split(",") if c}
    try:
        for rel in OWNED_FILES + (TRACKER,):
            if not (root / rel).is_file():
                raise ToolError(f"{root} is not a maos repo root: missing {rel}")
        xtask = find_xtask(root, args.xtask)
        m = measure(root, xtask)
        base, base_why = measure_base(root, xtask, args.base, args.base_tree)
        if base is None:
            print(f"repin: no pre-landing baseline ({base_why}); reserve-vs-exact rows cannot be told "
                  "apart, so measured < pin rows are left alone (use --lower CRATE)", file=sys.stderr)
        table, want, drift, refusals = analyze(root, m, base, adjudicated, lower, args.note)
    except ToolError as e:
        print(f"repin: ERROR: {e}", file=sys.stderr)
        return EXIT_TOOL

    print_table(table)
    print(f"\nmeasured: kernel physical {m['physical']} / tokei {m['per_crate'].get(KERNEL_CRATE)}, "
          f"aggregate {m['aggregate']}, {len(m['digests'])} kernel files")

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
        if "|" in note:
            raise Refuse("--note must not contain `|` (it is the HISTORY segment separator the d11 ratchet splits on)")
        story_key = resolve_story(root, args.story)
    except Refuse as r:
        print("\nREFUSED -- a human decision is needed; nothing was written:", file=sys.stderr)
        print(f"  - {r}", file=sys.stderr)
        return EXIT_HUMAN

    try:
        new_texts = build_texts(root, want, story_key, note, base)
        validate_new_texts(root, new_texts, want, story_key)
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
        rc_l, l = gate_json(xtask, root, ["kloc-check", "--json"])
        if rc_k != 0 or not k.get("passed") or rc_l != 0 or not l.get("passed"):
            raise ToolError(f"post-write verification failed: check-kernel-baseline passed={k.get('passed')}, "
                            f"kloc-check passed={l.get('passed')} over_budget={l.get('over_budget')}")
    except (ToolError, OSError) as e:
        for rel, text in originals.items():
            write_atomic(root / rel, text)
        print(f"repin: ERROR: {e}; all files restored", file=sys.stderr)
        return EXIT_DRIFT
    print(f"\nAPPLIED to {len(new_texts)} file(s): {', '.join(sorted(new_texts))}")
    print("verified: check-kernel-baseline PASSED, kloc-check PASSED "
          f"(aggregate {l['aggregate']}); run `repin.py --check` for the full table")
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
