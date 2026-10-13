#!/usr/bin/env python3
"""Run checked tests in a disposable, asserted 17-2 systemd cgroup unit."""

import argparse
import errno
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import tempfile
import tomllib
import uuid

_RESULT = re.compile(
    r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;",
    re.MULTILINE,
)
_FORBIDDEN = ("cpu", "memory", "pids")
_DENIED = {errno.EPERM, errno.EACCES, errno.EROFS}


def check_test_output(text, exit_code, expected_counts):
    """Reject unsuccessful, skipped, vacuous, malformed, or reordered suites."""
    if exit_code != 0:
        raise ValueError(f"test command exited with status {exit_code}")
    # Serial libtest flushes `test <name> ... ` before the body runs, so a
    # test's own SKIP line can follow that prefix instead of starting the line.
    if re.search(r"^(?:test \S+ \.\.\. )?\s*SKIP\b", text, re.MULTILINE):
        raise ValueError("test output contains an explicit SKIP")
    suites = _RESULT.findall(text)
    expected = list(expected_counts)
    if len(suites) != len(expected):
        raise ValueError(f"expected {len(expected)} test summaries, found {len(suites)}")
    for index, (suite, count) in enumerate(zip(suites, expected), start=1):
        status, passed, failed, ignored = suite
        if int(count) <= 0:
            raise ValueError(f"expected suite {index} must select at least one test")
        if status != "ok" or int(passed) != int(count) or failed != "0" or ignored != "0":
            raise ValueError(
                f"suite {index} expected {count} passed, zero failed/ignored; "
                f"got {status}, {passed} passed, {failed} failed, {ignored} ignored"
            )


def _membership():
    for line in Path("/proc/self/cgroup").read_text().splitlines():
        if line.startswith("0::"):
            return line[3:]
    raise RuntimeError("process has no unified cgroup-v2 membership")


def _validate_unit(unit, mode):
    if os.geteuid() == 0:
        raise RuntimeError("the systemd unit must run as the non-root invoking user")
    membership = _membership()
    expected = f"/system.slice/{unit}.service"
    if membership != expected:
        raise RuntimeError(f"expected exact cgroup membership {expected}, got {membership}")
    mountinfo = Path("/proc/self/mountinfo").read_text().splitlines()
    if not any(len(fields := line.split(" - ")) == 2
               and fields[0].split()[4] == "/sys/fs/cgroup"
               and fields[1].split()[0] == "cgroup2" for line in mountinfo):
        raise RuntimeError("cgroup2 is not mounted at /sys/fs/cgroup")
    actual = subprocess.run(
        ["systemctl", "show", f"{unit}.service", "--property=Delegate", "--value"],
        check=True, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    ).stdout.strip()
    required = "yes" if mode == "delegated" else "no"
    if actual != required:
        raise RuntimeError(f"unit Delegate={actual!r}; required Delegate={required}")
    return Path("/sys/fs/cgroup") / membership.lstrip("/")


def _enable_positive_domain(root):
    if (root / "cgroup.type").read_text().strip() != "domain":
        raise RuntimeError("delegated service cgroup is not a domain")
    available = set((root / "cgroup.controllers").read_text().split())
    missing = set(_FORBIDDEN) - available
    if missing:
        raise RuntimeError(f"delegated domain lacks controllers: {', '.join(sorted(missing))}")
    daemon = root / "daemon"
    daemon.mkdir()
    (daemon / "cgroup.procs").write_text(f"{os.getpid()}\n")
    if (root / "cgroup.procs").read_text().strip():
        raise RuntimeError("delegated domain is not empty before controller enablement")
    (root / "cgroup.subtree_control").write_text("+cpu +memory +pids\n")
    enabled = set((root / "cgroup.subtree_control").read_text().split())
    missing = set(_FORBIDDEN) - enabled
    if missing:
        raise RuntimeError(f"failed to enable delegated controllers: {', '.join(sorted(missing))}")
    if _membership() != f"/{root.relative_to('/sys/fs/cgroup')}/daemon":
        raise RuntimeError("test process did not remain in the delegated daemon leaf")
    print(f"CGROUP-17-2 delegated membership={_membership()} controllers=cpu,memory,pids", flush=True)


def _assert_undelegated(root):
    mount = Path("/sys/fs/cgroup")
    candidates = []
    for candidate in (root, *root.parents):
        if not candidate.is_relative_to(mount):
            break
        enabled = set((candidate / "cgroup.subtree_control").read_text().split())
        # Every resource controller exposed to discovery is a possible selected
        # ancestor for a fixture requesting that individual cap.
        if enabled.intersection(_FORBIDDEN):
            candidates.append(candidate)
    if not candidates:
        raise RuntimeError("no resource-controller candidate ancestor for negative permission probes")
    for candidate in candidates:
        try:
            descriptor = os.open(candidate / "cgroup.procs", os.O_WRONLY)
        except OSError as error:
            if error.errno not in _DENIED:
                raise RuntimeError(f"unexpected common-placement probe error at {candidate}: {error}") from error
        else:
            os.close(descriptor)
            raise RuntimeError(f"unexpected common-placement access at eligible ancestor {candidate}")
        parents = [candidate]
        maos = candidate / "maos.slice"
        if maos.is_dir():
            parents.append(maos)
        for parent in parents:
            probe = parent / f"maos-17-2-probe-{uuid.uuid4().hex}"
            try:
                probe.mkdir()
            except OSError as error:
                if error.errno not in _DENIED:
                    raise RuntimeError(f"unexpected owned-subtree probe error at {parent}: {error}") from error
            else:
                try:
                    probe.rmdir()
                except OSError as error:
                    raise RuntimeError(f"cannot clean only the unexpected empty probe {probe}: {error}") from error
                raise RuntimeError(f"unexpected owned-subtree creation access at {parent}")
        print(f"CGROUP-17-2 negative ancestor={candidate} common-placement=denied owned-subtree=denied", flush=True)


def _checked_subprocess(arguments, expected_counts=None):
    process = subprocess.run(arguments, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    output = process.stdout
    sys.stdout.write(output)
    sys.stdout.flush()
    if expected_counts is not None:
        try:
            check_test_output(output, process.returncode, expected_counts)
        except ValueError:
            if os.environ.get("GITHUB_ACTIONS") == "true":
                detail = output[-2800:].replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
                print(f"::error title=17-2 actual canary failure::{detail}", flush=True)
            raise
    return process.returncode


def _require_rootless_substrate():
    required = (
        "MAOS_T3_RUNTIME",
        "MAOS_T3_IMAGE_LOCK_PATH",
        "MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX",
    )
    missing = [name for name in required if not os.environ.get(name)]
    if missing:
        raise RuntimeError(
            "rootless T3 canary lacks provisioned substrate: " + ", ".join(missing)
        )
    if os.environ["MAOS_T3_RUNTIME"] not in ("podman", "docker"):
        raise RuntimeError("rootless T3 canary requires MAOS_T3_RUNTIME=podman or docker")


def _check_t3_runtime():
    # This service runs with the caller's UID, XDG_RUNTIME_DIR, and D-Bus
    # address; successful user-manager access proves those were transported.
    subprocess.run(
        ["systemctl", "--user", "show-environment"],
        check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
    )
    runtime = os.environ["MAOS_T3_RUNTIME"]
    lock_path = Path(os.environ["MAOS_T3_IMAGE_LOCK_PATH"])
    lock = json.loads(lock_path.read_text())
    if not lock or not lock[0].get("entries"):
        raise RuntimeError("provisioned T3 image lock contains no image entry")
    image = lock[0]["entries"][0]["image_uri"]
    environment = dict(os.environ)
    with tempfile.TemporaryDirectory(prefix="maos-17-2-t3-home-") as temporary_home:
        if runtime == "podman":
            storage = lock_path.with_name("storage.conf").resolve(strict=True)
            config = Path(temporary_home) / ".config" / "containers"
            config.mkdir(parents=True)
            (config / "storage.conf").symlink_to(storage)
            environment["HOME"] = temporary_home
            expected_store = tomllib.loads(storage.read_text()).get("storage", {})
            actual_store = json.loads(subprocess.run(
                [runtime, "info", "--format", "{{json .Store}}"],
                check=True, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                env=environment,
            ).stdout)
            fields = {
                "graphRoot": "graphroot",
                "runRoot": "runroot",
                "graphDriverName": "driver",
            }
            for actual, configured in fields.items():
                if expected_store.get(configured) is None or actual_store.get(actual) != expected_store[configured]:
                    raise RuntimeError(f"Podman does not use the provisioned {actual} store")
        digests = subprocess.run(
            [runtime, "image", "inspect", "--format", "{{json .RepoDigests}}", image],
            check=True, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            env=environment,
        ).stdout
        if image not in json.loads(digests):
            raise RuntimeError("rootless runtime lacks the exact provisioned locked image")
    print(
        f"T3-ROOTLESS-CANARY uid={os.geteuid()} user-manager=reachable "
        f"runtime={runtime} locked-image=present store=provisioned",
        flush=True,
    )


def _inside(args):
    try:
        root = _validate_unit(args.unit, args.mode)
        if args.mode == "delegated":
            _enable_positive_domain(root)
        else:
            _assert_undelegated(root)
        print(
            f"CGROUP-17-2 mode={args.mode} uid={os.geteuid()} unit={args.unit} "
            f"membership={_membership()} Delegate={'yes' if args.mode == 'delegated' else 'no'}",
            flush=True,
        )
        if args.rootless_canary:
            if args.mode != "delegated":
                raise RuntimeError("the rootless T3 canary requires the delegated mode")
            _require_rootless_substrate()
            _check_t3_runtime()
            canary = [
                "cargo", "test", "--locked", "--offline", "-p", "maos-bin",
                "--features", "network,wasm-host", "--test", "smoke_cli_wrapper_8_12",
                "maos_run_cli_wrapper_worker_spawns_real_subprocess", "--", "--exact",
                "--nocapture", "--test-threads=1",
            ]
            _checked_subprocess(canary, [1])
            proof = f"uid={os.geteuid()} user-manager=reachable store=provisioned tests=1 passed,0 failed,0 ignored"
            if os.environ.get("GITHUB_ACTIONS") == "true":
                print(f"::notice title=17-2 actual rootless canary::{proof}", flush=True)
            else:
                print(f"T3-ROOTLESS-CANARY {proof}", flush=True)
        status = subprocess.run(args.command, check=False).returncode
        return status if status >= 0 else 128 - status
    except (OSError, RuntimeError, subprocess.SubprocessError, ValueError) as error:
        print(f"cgroup-ceiling harness: {error}", file=sys.stderr, flush=True)
        if os.environ.get("GITHUB_ACTIONS") == "true":
            detail = str(error)
            stderr = getattr(error, "stderr", None)
            if stderr:
                detail += "\n" + (stderr.decode(errors="replace") if isinstance(stderr, bytes) else stderr)
            detail = detail[:2800].replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
            print(f"::error title=17-2 actual harness refusal::{detail}", flush=True)
        return 1


def _environment(mode):
    required = ("PATH", "HOME")
    missing = [key for key in required if key not in os.environ]
    if missing:
        raise ValueError(f"required caller environment is missing: {', '.join(missing)}")
    names = set(required) | {
        key for key in os.environ
        if key.startswith("CARGO_") or key.startswith("RUSTUP_")
    }
    names.update(("RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"))
    names.update(("CI", "GITHUB_ACTIONS", "MAOS_SECRETS_BACKEND"))
    names.update((
        "MAOS_T3_RUNTIME", "MAOS_T3_IMAGE_LOCK_PATH",
        "MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX", "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
    ))
    values = {key: os.environ[key] for key in names if key in os.environ}
    values["MAOS_17_2_CGROUP_MODE"] = mode
    return values


def _systemd_run(args):
    if os.geteuid() == 0:
        raise ValueError("invoke the harness as the non-root CI user; sudo is used only for systemd setup")
    if not re.fullmatch(r"maos-17-2-[A-Za-z0-9_-]+", args.unit):
        raise ValueError("unit must be a named maos-17-2-* service without a unit-type suffix")
    if args.timeout <= 0:
        raise ValueError("timeout must be positive seconds")
    if args.rootless_canary and args.mode != "delegated":
        raise ValueError("the rootless T3 canary requires the delegated mode")
    if not args.command:
        raise ValueError("a command is required after --")
    environment = _environment(args.mode)
    # A per-invocation name means cleanup can only ever stop this run's unit,
    # never a concurrent run that was given the same requested name.
    unit = f"{args.unit}-{uuid.uuid4().hex[:12]}"
    command = [
        sys.executable, str(Path(__file__).resolve()), "_inside",
        "--mode", args.mode, "--unit", unit,
    ]
    if args.rootless_canary:
        command.append("--rootless-canary")
    command.extend(("--", *args.command))
    invocation = [
        "sudo", "systemd-run", f"--unit={unit}", "--wait", "--pipe", "--collect",
        "--expand-environment=no", f"--uid={os.getuid()}", f"--gid={os.getgid()}",
        f"--property=Delegate={'yes' if args.mode == 'delegated' else 'no'}",
        f"--property=RuntimeMaxSec={args.timeout}", "--property=TimeoutStopSec=10",
        "--property=KillMode=control-group", f"--property=WorkingDirectory={Path.cwd()}",
    ]
    invocation.extend(f"--setenv={key}={value}" for key, value in sorted(environment.items()))
    invocation.extend(command)
    child = subprocess.Popen(invocation)
    received_signal = []
    signal_stop = []

    def forward(signum, _frame):
        if not received_signal:
            received_signal.append(signum)
            try:
                child.send_signal(signum)
            except ProcessLookupError:
                pass
            try:
                signal_stop.append(subprocess.Popen(
                    ["sudo", "systemctl", "stop", f"{unit}.service"],
                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                ))
            except OSError:
                pass
    previous = {signum: signal.signal(signum, forward) for signum in (signal.SIGINT, signal.SIGTERM)}
    stop_failed = False
    try:
        status = child.wait()
    finally:
        for signum, handler in previous.items():
            signal.signal(signum, handler)
        for stopped in signal_stop:
            try:
                stopped.wait(timeout=20)
            except subprocess.TimeoutExpired:
                stop_failed = True
                stopped.kill()
                stopped.wait()
        try:
            stopped = subprocess.run(
                ["sudo", "systemctl", "stop", f"{unit}.service"],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=20,
            )
            if stopped.returncode != 0:
                active = subprocess.run(
                    ["sudo", "systemctl", "is-active", "--quiet", f"{unit}.service"],
                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10,
                )
                stop_failed = active.returncode == 0
        except (OSError, subprocess.SubprocessError):
            stop_failed = True
    if received_signal:
        return 128 + received_signal[0]
    if stop_failed and status == 0:
        print("cgroup-ceiling harness: failed to stop the bounded system unit", file=sys.stderr)
        return 1
    return status if status >= 0 else 128 - status


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="action", required=True)
    run = subparsers.add_parser("run", help="run a command in an asserted transient system unit")
    run.add_argument("--mode", choices=("delegated", "undelegated"), required=True)
    run.add_argument("--unit", required=True)
    run.add_argument("--timeout", type=int, required=True, help="unit RuntimeMaxSec in seconds")
    run.add_argument("--rootless-canary", action="store_true")
    run.add_argument("command", nargs=argparse.REMAINDER)
    check = subparsers.add_parser("check-log", help="require exact successful Rust test-suite counts")
    check.add_argument("--exit-code", type=int, required=True)
    check.add_argument("--expected-count", type=int, action="append", required=True)
    check.add_argument("log", type=Path)
    inside = subparsers.add_parser("_inside", help=argparse.SUPPRESS)
    inside.add_argument("--mode", choices=("delegated", "undelegated"), required=True)
    inside.add_argument("--unit", required=True)
    inside.add_argument("--rootless-canary", action="store_true")
    inside.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    if args.action == "check-log":
        try:
            text = args.log.read_text(errors="replace")
            check_test_output(text, args.exit_code, args.expected_count)
        except (OSError, ValueError) as error:
            print(f"cgroup-ceiling check-log: {error}", file=sys.stderr)
            return 1
        proof = f"log={args.log.name} exit={args.exit_code} suites={_RESULT.findall(text)}"
        if os.environ.get("GITHUB_ACTIONS") == "true":
            print(f"::notice title=17-2 checked execution::{proof}", flush=True)
        else:
            print(f"CHECKED-17-2 {proof}", flush=True)
        return 0
    if args.command and args.command[0] == "--":
        args.command = args.command[1:]
    if args.action == "_inside":
        return _inside(args)
    try:
        return _systemd_run(args)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"cgroup-ceiling harness: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
