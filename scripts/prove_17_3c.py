#!/usr/bin/env python3
"""Run the first-party TS CLI under an already delegated Linux cgroup.

Produces actual launch/audit/resource evidence; never provisions controllers or
weakens the manifest. A missing delegation, receipt, or real process is a failure.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import sqlite3
import subprocess
import sys
import threading
import time
import tomllib

# (call_type, intent) rows `maosctl audit query --spirit` must attribute to the
# admitted Spirit's PID: the frame authorization, the guest's own TaskAssign,
# the runner's terminal receipt, and the unload.
EXPECTED_AUDIT_ROWS = [
    ("governance.event", "spirit.frame.authorization"),
    ("task.assign", "standard"),
    ("capability.invocation", "cli.subprocess.exit"),
    ("capability.invocation", "lifecycle.unload"),
]


def check(condition, message):
    """Fail the proof explicitly; unlike `assert`, survives `python -O`."""
    if not condition:
        print(f"17-3c proof FAILED: {message}", file=sys.stderr)
        raise SystemExit(1)


def first(items, message):
    for item in items:
        return item
    check(False, message)


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def argv_value(argv, flag):
    index = argv.index(flag) if flag in argv else -1
    return argv[index + 1] if 0 <= index < len(argv) - 1 else None


def source_constant(path, pattern):
    """The single integer literal (`a * b` products allowed) `pattern` captures in `path`."""
    matches = re.findall(pattern, path.read_text())
    check(len(matches) == 1, f"{path}: expected exactly one match for {pattern!r}, found {len(matches)}")
    value = 1
    for factor in matches[0].split("*"):
        value *= int(factor.strip().replace("_", ""))
    return value


def note_distinct(sample, field, value):
    if value not in sample[field]:
        sample[field].append(value)


def sample_runner(daemon, runner, stopped, observations):
    """Observe the real child PID and its enforced limits while it is alive."""
    while not stopped.wait(0.05):
        for task in Path(f"/proc/{daemon}/task").glob("*/children"):
            try:
                children = task.read_text().split()
            except OSError:
                continue
            for child in children:
                proc = Path(f"/proc/{child}")
                try:
                    if (proc / "exe").resolve(strict=True) != runner:
                        continue
                    relative = next(line[3:] for line in (proc / "cgroup").read_text().splitlines()
                                    if line.startswith("0::"))
                    cgroup = Path("/sys/fs/cgroup") / relative.lstrip("/")
                    sample = observations.setdefault(int(child), {
                        "child_pid": int(child), "cgroup": str(cgroup),
                        "cpu_max": (cgroup / "cpu.max").read_text().strip(),
                        "memory_max": (cgroup / "memory.max").read_text().strip(),
                        "argv": (proc / "cmdline").read_bytes().decode().rstrip("\0").split("\0"),
                        "vm_peak_virtual_kib": 0, "vm_hwm_rss_kib": 0,
                        "cgroup_memory_peak_bytes": None,
                        "seccomp_modes": [], "no_new_privs": [], "seccomp_filters": [],
                    })
                    status = dict(line.split(":", 1) for line in (proc / "status").read_text().splitlines()
                                  if ":" in line)
                    for key, field in [("VmPeak", "vm_peak_virtual_kib"), ("VmHWM", "vm_hwm_rss_kib")]:
                        if key in status:
                            sample[field] = max(sample[field], int(status[key].split()[0]))
                    for key, field in [("Seccomp", "seccomp_modes"), ("NoNewPrivs", "no_new_privs"),
                                       ("Seccomp_filters", "seccomp_filters")]:
                        note_distinct(sample, field, status[key].strip() if key in status else None)
                except (OSError, StopIteration, ValueError):
                    # The real child can exit between procfs reads. Missing
                    # observations still fail the final proof, never pass it.
                    continue
                try:
                    # Monotonic while the cgroup lives; the last read before
                    # terminal cleanup removes the cgroup is its peak.
                    peak = int((cgroup / "memory.peak").read_text())
                    sample["cgroup_memory_peak_bytes"] = max(sample["cgroup_memory_peak_bytes"] or 0, peak)
                except (OSError, ValueError):
                    continue


def audit_query(maosctl, env, spirit, pid):
    """Run the operator surface (`maosctl audit query --spirit`) in both formats.

    Returns the archived record; `missing` lists EXPECTED_AUDIT_ROWS absent
    from the plain table's rows attributed to `pid`.
    """
    record = {"maosctl": str(maosctl), "maosctl_sha256": digest(maosctl), "spirit": spirit,
              "spirit_pid": pid, "queries": {}}
    for fmt in ("ndjson", "plain"):
        command = [str(maosctl), "audit", "query", "--spirit", spirit, "--format", fmt]
        result = subprocess.run(command, env=env, capture_output=True, text=True)
        record["queries"][fmt] = {"command": command, "exit_code": result.returncode,
                                  "stdout": result.stdout, "stderr": result.stderr}
    rows = []
    for line in record["queries"]["plain"]["stdout"].splitlines()[1:]:
        fields = line.split(None, 5)
        if len(fields) == 6 and fields[2].isdigit():
            rows.append({"spirit_pid": int(fields[2]), "call_type": fields[3], "intent": fields[5]})
    attributed = {(row["call_type"], row["intent"]) for row in rows if row["spirit_pid"] == pid}
    record["present"] = [list(row) for row in EXPECTED_AUDIT_ROWS if row in attributed]
    record["missing"] = [list(row) for row in EXPECTED_AUDIT_ROWS if row not in attributed]
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--maosctl", type=Path, help="defaults to the maosctl beside --binary")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected-os")
    parser.add_argument("--expected-arch", choices=["x86_64", "aarch64"])
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    manifest = root / "examples/example-spirit-ts/manifest.toml"
    declared = tomllib.loads(manifest.read_text())
    binary = args.binary.resolve(strict=True)
    runner = binary.with_name("maos-wasm-runner").resolve(strict=True)
    maosctl = (args.maosctl or binary.with_name("maosctl")).resolve(strict=True)
    component = (manifest.parent / declared["class"]["artifact"]).resolve(strict=True)
    # Declared, not observed: the host's default fuel (manifest launches pass no
    # `fuel` form_config) and the runner's per-Store guest memory limit.
    declared_fuel = source_constant(root / "crates/maos-bin/src/main.rs",
                                    r"WasmHostConfig::new\(\s*runner_path,\s*([0-9_]+),?\s*\)")
    declared_guest_memory = source_constant(root / "crates/maos-wasm-host/src/host_state.rs",
                                            r"\.memory_size\(([0-9_ *]+)\)")
    declared_time_cap = declared["budget"]["time_cap_seconds"]
    release = dict(line.split("=", 1) for line in Path("/etc/os-release").read_text().splitlines()
                   if "=" in line and not line.startswith("#"))
    os_version = release.get("VERSION_ID", "").strip('"')
    arch = platform.machine()
    if args.expected_os:
        check(release.get("ID", "").strip('"') == "ubuntu" and os_version == args.expected_os, release)
    if args.expected_arch:
        check(arch == args.expected_arch, arch)
    check(declared["resources"] == {"cpu_max_pct": 10, "memory_max_mb": 768}, declared["resources"])
    check(declared_time_cap == 360, declared_time_cap)
    check(declared["sandbox"]["tier"] == "T2", declared["sandbox"])
    args.output.mkdir(parents=True, exist_ok=False)
    home = args.output / "home"
    home.mkdir()
    env = {key: value for key, value in os.environ.items() if not key.startswith("MAOS_")}
    env.update(HOME=str(home.resolve()), MAOS_HOME=str((home / "maos").resolve()),
               XDG_CONFIG_HOME=str((home / "config").resolve()),
               XDG_DATA_HOME=str((home / "data").resolve()), MAOS_NOTIFY_DISABLE="1")
    initialized = subprocess.run([str(binary), "init"], env=env, capture_output=True, text=True)
    (args.output / "init.stdout").write_text(initialized.stdout)
    (args.output / "init.stderr").write_text(initialized.stderr)
    check(initialized.returncode == 0, initialized.stderr)
    started = time.monotonic()
    process = subprocess.Popen([str(binary), "run", str(manifest), "--once"], env=env,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    stopped = threading.Event()
    observations = {}
    sampler = threading.Thread(target=sample_runner, args=(process.pid, runner, stopped, observations))
    sampler.start()
    stdout = stderr = ""
    try:
        stdout, stderr = process.communicate(timeout=declared_time_cap + 40)
    except subprocess.TimeoutExpired:
        process.kill()
        stdout, stderr = process.communicate()
        raise
    finally:
        stopped.set()
        sampler.join()
        (args.output / "launch.stdout").write_text(stdout)
        (args.output / "launch.stderr").write_text(stderr)
        (args.output / "process-observations.json").write_text(json.dumps(observations, indent=2) + "\n")
    provenance = {
        "platform": {"os_release": release, "arch": arch, "kernel": platform.release()},
        "exit_code": process.returncode, "elapsed_seconds": time.monotonic() - started,
        "manifest_sha256": digest(manifest), "component_sha256": digest(component),
        "daemon_sha256": digest(binary), "runner_sha256": digest(runner),
        "maosctl_sha256": digest(maosctl),
        "declared_resources": declared["resources"], "declared_time_cap_seconds": declared_time_cap,
        "declared_instruction_fuel": declared_fuel, "declared_guest_memory_bytes": declared_guest_memory,
    }
    (args.output / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    check(process.returncode == 0, stderr)
    events = []
    for line in stdout.splitlines():
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    loaded = first((event for event in events if event.get("event") == "spirit_loaded"),
                   "no spirit_loaded event on stdout")
    check(loaded.get("ready") is True and loaded.get("spirit_id") == declared["class"]["name"], loaded)
    check(loaded.get("child_pid") in observations, f"child {loaded.get('child_pid')} never observed: {observations}")
    sample = observations[loaded["child_pid"]]
    quota, period = map(int, sample["cpu_max"].split())
    check(quota * 100 == period * declared["resources"]["cpu_max_pct"], sample)
    check(int(sample["memory_max"]) == declared["resources"]["memory_max_mb"] * 1024 * 1024, sample)
    check(argv_value(sample["argv"], "--fuel") == str(declared_fuel), sample)
    observed_component = argv_value(sample["argv"], "--component")
    check(observed_component is not None and Path(observed_component).resolve() == component
          and digest(Path(observed_component)) == provenance["component_sha256"],
          f"observed --component {observed_component!r} is not the hashed {component}")
    check(sample["seccomp_modes"] == ["2"], f"runner Seccomp modes {sample['seccomp_modes']}, want ['2']")
    check(sample["no_new_privs"] == ["1"], f"runner NoNewPrivs {sample['no_new_privs']}, want ['1']")
    check(sample["vm_hwm_rss_kib"] > 0, sample)
    deadline = time.monotonic() + 10
    while Path(sample["cgroup"]).exists() and time.monotonic() < deadline:
        time.sleep(0.1)
    check(not Path(sample["cgroup"]).exists(), "owned child cgroup survived terminal cleanup")
    database = home / "maos/audit/transparency.sqlite"
    with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
        connection.row_factory = sqlite3.Row
        rows = [dict(row) for row in connection.execute(
            "SELECT spirit_pid,from_spirit_id,to_spirit_id,kind,intent,payload_redacted,origin FROM transparency_log ORDER BY rowid")]
    pid = loaded["pid"]
    attributed = [row for row in rows if row["spirit_pid"] == pid]
    intents = {row["intent"] for row in attributed}
    check({"lifecycle.admit", "lifecycle.load", "lifecycle.start", "spirit.frame.authorization",
           "cli.subprocess.exit", "lifecycle.unload", "planned_unload"} <= intents, intents)
    check(any(row["spirit_pid"] == 0 and row["from_spirit_id"] == "operator" and row["kind"] == 0 for row in rows),
          "no operator TaskAssign row")
    check(any(row["kind"] == 0 and row["from_spirit_id"] == loaded["spirit_id"]
              and row["intent"] == "standard" and row["origin"] == 1 for row in attributed),
          "no attributed Standard/SpiritAuto guest row")
    authorization = json.loads(first((row["payload_redacted"] for row in attributed
                                      if row["intent"] == "spirit.frame.authorization"), "no authorization row"))
    check(authorization.get("allowed") is True and authorization.get("trusted_sender") == loaded["spirit_id"],
          authorization)
    check(authorization.get("spirit_pid") == pid
          and authorization.get("claims", {}).get("from", {}).get("spirit_id") == "operator", authorization)
    terminal = json.loads(first((row["payload_redacted"] for row in attributed
                                 if row["intent"] == "cli.subprocess.exit"), "no terminal receipt row"))
    check(terminal.get("is_crash") is False, terminal)
    check(not {"task.stalled", "lifecycle.crash"} & intents, intents)
    check(not any(intent.startswith("sandbox.block.") for intent in intents), intents)
    queried = audit_query(maosctl, env, loaded["spirit_id"], pid)
    (args.output / "audit-query.json").write_text(json.dumps(queried, indent=2) + "\n")
    for fmt, result in queried["queries"].items():
        check(result["exit_code"] == 0, f"maosctl audit query --format {fmt} exited {result['exit_code']}: "
                                        f"{result['stderr'].strip()}")
    check(not queried["missing"], f"maosctl audit query lacks attributed rows {queried['missing']}")
    proof = {"loaded": loaded, "enforced_resources": sample, "attributed_intents": sorted(intents),
             "authorization": authorization, "terminal": terminal, "database": str(database),
             "audit_query_rows": queried["present"]}
    (args.output / "accepted-proof.json").write_text(json.dumps(proof, indent=2) + "\n")
    print(f"TS-ADMITTED-PROOF os={os_version} arch={arch} pid={pid} child_pid={loaded['child_pid']} "
          f"cpu={declared['resources']['cpu_max_pct']} memory_mib={declared['resources']['memory_max_mb']} "
          f"audit=attributed terminal=clean")


if __name__ == "__main__":
    main()
