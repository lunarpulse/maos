#!/usr/bin/env python3
import json
import os
import socket
import subprocess
import tempfile

ROOT = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
RUNNER = f"{ROOT}/target/release/spike-runner"
SERVICE = f"{ROOT}/target/release/recall-service"
DRIVER = f"{ROOT}/target/release/frame-driver"
GUEST = f"{ROOT}/target/wasm32-wasip2/release/story_17_3a_recall_guest.wasm"
ATTACKER = f"{ROOT}/spikes/story-17-3a-wasm-recall/host/fd_attacker.py"

left, right = socket.socketpair()
runner_fd = left.fileno()
service_fd = right.fileno()
with tempfile.TemporaryDirectory() as directory:
    ids_path = f"{directory}/ids.json"
    runner = subprocess.Popen(
        [RUNNER, "--component", GUEST, "--fuel", "1000000000", "--recall-fd", str(runner_fd)],
        cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, pass_fds=(runner_fd,),
    )
    service = subprocess.Popen(
        [SERVICE, "--fd", str(service_fd), "--caller-pid", str(runner.pid),
         "--seed-pids", f"{runner.pid},424242", "--ids-out", ids_path],
        cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, pass_fds=(service_fd,),
    )
    left.close()
    right.close()
    ready = service.stdout.readline().strip()
    target = os.readlink(f"/proc/{runner.pid}/fd/{runner_fd}")
    attacker = subprocess.run([ATTACKER, str(runner.pid), str(runner_fd)], cwd=ROOT, text=True,
                              stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=30)
    ids = json.load(open(ids_path))
    inbound = subprocess.check_output([DRIVER, "encode", "--data", ids[str(runner.pid)]], cwd=ROOT)
    runner_out, runner_err = runner.communicate(inbound, timeout=30)
    service_out, service_err = service.communicate(timeout=30)
    decoded = subprocess.run([DRIVER, "decode"], cwd=ROOT, input=runner_out,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout.decode()
    yama = open("/proc/sys/kernel/yama/ptrace_scope").read().strip()
print(f"E15-A6: this probe reads live inherited descriptors, /proc, pidfd_getfd, yama, and a running relay; it does not set the access result.")
print(f"runner_pid={runner.pid} runner_fd={runner_fd} target={target} yama_ptrace_scope={yama}")
print(f"service_ready={ready}")
print(f"attacker_rc={attacker.returncode} attacker={attacker.stdout.strip()}")
print(f"runner_rc={runner.returncode} service_rc={service.returncode}")
print("-- decoded control relay --")
print(decoded, end="")
print("-- runner stderr --")
print(runner_err.decode(), end="")
print("-- service stderr --")
print(service_err, end="")
if runner.returncode or service.returncode or attacker.returncode:
    raise SystemExit("FD vector probe failed")
