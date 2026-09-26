#!/usr/bin/env python3
import json
import os
import subprocess
import tempfile
import time

ROOT = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
RUNNER = f"{ROOT}/target/release/spike-runner"
SERVICE = f"{ROOT}/target/release/recall-service"
DRIVER = f"{ROOT}/target/release/frame-driver"
GUEST = f"{ROOT}/target/wasm32-wasip2/release/story_17_3a_recall_guest.wasm"

with tempfile.TemporaryDirectory() as directory:
    os.chmod(directory, 0o700)
    socket_path = f"{directory}/recall.sock"
    ids_path = f"{directory}/ids.json"
    runner = subprocess.Popen([RUNNER, "--component", GUEST, "--fuel", "1000000000", "--recall-socket", socket_path],
                              cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    service = subprocess.Popen([SERVICE, "--socket", socket_path, "--caller-pid", str(runner.pid),
                                "--seed-pids", f"{runner.pid},424242", "--ids-out", ids_path, "--one-shot"],
                               cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    ready = service.stdout.readline().strip()
    frame_id = json.load(open(ids_path))[str(runner.pid)]
    runner.stdin.write(subprocess.check_output([DRIVER, "encode", "--data", frame_id], cwd=ROOT))
    runner.stdin.flush()  # Keep stdin and the runner's service stream open after recall/fetch.
    start = time.monotonic()
    service_stdout, service_stderr = service.communicate(timeout=5)
    elapsed_ms = int((time.monotonic() - start) * 1000)
    runner.terminate()
    _, runner_stderr = runner.communicate(timeout=10)
print("E15-A6: this probe reads a live one-shot service after a runner completes recall/fetch but keeps both streams open; it does not set the termination result.")
print(f"service_ready={ready}")
print(f"service_rc={service.returncode} idle_exit_ms={elapsed_ms} runner_terminated_rc={runner.returncode}")
print("-- service stderr --")
print(service_stderr, end="")
print("-- runner stderr --")
print(runner_stderr.decode(), end="")
if service.returncode or "capability-invocation-audit-count=2" not in service_stderr or "one-shot-idle-timeout" not in service_stderr or elapsed_ms > 2500:
    raise SystemExit("one-shot service did not terminate after idle timeout")
