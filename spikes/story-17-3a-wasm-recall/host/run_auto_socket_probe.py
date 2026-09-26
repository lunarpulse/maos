#!/usr/bin/env python3
import json
import subprocess
import tempfile

ROOT = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
RUNNER = f"{ROOT}/target/release/spike-runner"
SERVICE = f"{ROOT}/target/release/recall-service"
DRIVER = f"{ROOT}/target/release/frame-driver"
GUEST = f"{ROOT}/target/wasm32-wasip2/release/story_17_3a_recall_guest.wasm"

with tempfile.TemporaryDirectory() as directory:
    ids_path = f"{directory}/ids.json"
    service = subprocess.Popen(
        [SERVICE, "--socket", "auto", "--caller-pid", "0", "--seed-pids", "0,424242",
         "--ids-out", ids_path, "--no-peer-pid-check", "--one-shot"],
        cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    ready = service.stdout.readline().strip()
    socket_path = ready.split("socket=", 1)[1].split(" caller-pid=", 1)[0]
    ids = json.load(open(ids_path))
    runner = subprocess.Popen(
        [RUNNER, "--component", GUEST, "--fuel", "1000000000", "--recall-socket", socket_path],
        cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    inbound = subprocess.check_output([DRIVER, "encode", "--data", ids["0"]], cwd=ROOT)
    runner_out, runner_err = runner.communicate(inbound, timeout=30)
    service_out, service_err = service.communicate(timeout=30)
    decoded = subprocess.run([DRIVER, "decode"], cwd=ROOT, input=runner_out,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout.decode()
print("E15-A6: this probe reads a runtime-selected service socket path and a runner that receives it; it does not set the selected path.")
print(f"service_ready={ready}")
print(f"launcher_passed_socket={socket_path}")
print(f"runner_rc={runner.returncode} service_rc={service.returncode}")
print("-- decoded relay --")
print(decoded, end="")
print("-- runner stderr --")
print(runner_err.decode(), end="")
print("-- service stderr --")
print(service_err, end="")
if runner.returncode or service.returncode:
    raise SystemExit("auto socket handoff probe failed")
