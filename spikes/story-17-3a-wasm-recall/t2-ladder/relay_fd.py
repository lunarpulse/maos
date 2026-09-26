#!/usr/bin/env python3
import json
import os
import pathlib
import socket
import subprocess
import sys
import tempfile
import time

repo = pathlib.Path(__file__).resolve().parents[3]
ladder = repo / "target/debug/t2-ladder"
spike_runner = repo / "target/release/spike-runner"
recall_service = repo / "target/release/recall-service"
frame_driver = repo / "target/release/frame-driver"
guest_wasm = repo / "target/wasm32-wasip2/release/story_17_3a_recall_guest.wasm"
seccomp = os.environ.get("T2_SECCOMP", "off")
seccomp_order = os.environ.get("T2_SECCOMP_ORDER", "verbatim")
extra = os.environ.get("T2_RUNNER_EXTRAS", "")
runner_flags = os.environ.get("SPIKE_RUNNER_FLAGS", "").split()
precompiled = os.environ.get("SPIKE_PRECOMPILED", "")
print(f"UTC {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}")
print("+ git rev-parse HEAD")
print(subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip())
print("PROBE_INPUTS tree/runtime: real spike-runner, Rust recall guest, connected AF_UNIX socketpair, recall-service, and seeded log frames; not only probe-created state.")

with tempfile.TemporaryDirectory(prefix="maos-q4b-fd-") as temp:
    temp_path = pathlib.Path(temp)
    pid_out = temp_path / "runner.pid"
    ids_out = temp_path / "ids.json"
    runner_out = temp_path / "runner.out"
    runner_err = temp_path / "runner.err"
    service_out = temp_path / "service.out"
    service_err = temp_path / "service.err"
    read_fd, write_fd = os.pipe()
    guest_end, service_end = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
    t2_reads = ["--read", str(guest_wasm)]
    runner_args = [str(spike_runner), "--component", str(guest_wasm), "--recall-fd", str(guest_end.fileno()), *runner_flags]
    if precompiled:
        t2_reads.extend(["--read", precompiled])
        runner_args.extend(["--precompiled", precompiled])
    args = [
        str(ladder), "--landlock", "rights", "--execute-loader", "--seccomp", seccomp,
        "--seccomp-order", seccomp_order, *t2_reads, "--stdin", f"/proc/self/fd/{read_fd}",
        "--inherit-fd", str(guest_end.fileno()), "--pid-out", str(pid_out), "--", *runner_args,
    ]
    if seccomp == "candidate":
        args[args.index("--read"):args.index("--read")] = ["--extra", extra]
    print("+ " + " ".join(args))
    with runner_out.open("wb") as stdout, runner_err.open("wb") as stderr:
        runner = subprocess.Popen(args, pass_fds=(read_fd, guest_end.fileno()), stdout=stdout, stderr=stderr)
    os.close(read_fd)
    guest_end.close()
    for _ in range(100):
        if pid_out.exists() and pid_out.stat().st_size:
            break
        if runner.poll() is not None:
            raise RuntimeError(f"ladder exited before publishing runner pid: rc={runner.returncode}")
        time.sleep(0.05)
    runner_pid = int(pid_out.read_text(encoding="utf-8"))
    service_args = [
        str(recall_service), "--fd", str(service_end.fileno()), "--caller-pid", str(runner_pid),
        "--seed-pids", f"{runner_pid},424242", "--ids-out", str(ids_out),
    ]
    print("+ " + " ".join(service_args))
    with service_out.open("wb") as stdout, service_err.open("wb") as stderr:
        service = subprocess.Popen(service_args, pass_fds=(service_end.fileno(),), stdout=stdout, stderr=stderr)
    service_end.close()
    for _ in range(100):
        if ids_out.exists() and ids_out.stat().st_size:
            break
        if service.poll() is not None:
            raise RuntimeError(f"service exited before seeding: rc={service.returncode}")
        time.sleep(0.05)
    frame_id = json.loads(ids_out.read_text(encoding="utf-8"))[str(runner_pid)]
    print(f"SEED_FRAME_ID={frame_id}")
    print(f"+ {frame_driver} encode --data {frame_id} > runner stdin")
    with os.fdopen(write_fd, "wb", closefd=True) as writer:
        subprocess.run([str(frame_driver), "encode", "--data", frame_id], stdout=writer, check=True)
    runner_rc = runner.wait(timeout=30)
    service_rc = service.wait(timeout=30)
    print(f"LADDER_RC={runner_rc} SERVICE_RC={service_rc}")
    raw = runner_out.read_bytes()
    start = raw.index(b"Content-Length: ")
    header_end = raw.index(b"\r\n\r\n", start) + 4
    body_len = int(raw[start:header_end].split(b":", 1)[1].strip())
    frame = raw[start:header_end + body_len]
    print("+ frame-driver decode < inherited-fd runner output")
    decoded = subprocess.run([str(frame_driver), "decode"], input=frame, capture_output=True, check=True)
    sys.stdout.write(decoded.stdout.decode("utf-8", "replace"))
    sys.stdout.write(decoded.stderr.decode("utf-8", "replace"))
    print("+ cat service stdout and stderr")
    sys.stdout.write(service_out.read_text(encoding="utf-8", errors="replace"))
    sys.stdout.write(service_err.read_text(encoding="utf-8", errors="replace"))
    print("+ cat ladder stderr and RESULT")
    sys.stdout.write(runner_err.read_text(encoding="utf-8", errors="replace"))
    for line in raw.decode("utf-8", "replace").splitlines():
        if line.startswith("RESULT "):
            print(line)
    if runner_rc != 0 or service_rc != 0:
        raise RuntimeError("inherited-fd relay did not complete cleanly")
