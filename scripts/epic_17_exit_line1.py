#!/usr/bin/env python3
"""Epic-17 exit lines 1 and 1b (Story 17-1 AC1), on a declared-capable runner.

Runs the real T3 Worker topology and its audit query exactly as the exit lines
state, then re-reads the Transparency Log independently:

    maos run spirits/topologies/worker-sandbox.toml --once
    maos audit query --spirit worker --format plain

The probing Worker (`worker`) must leave exactly three kind-8 `sandbox.block`
rows, all Kernel-origin: two filesystem rows from its stderr marker and one
network row from the proxy's confirmed refusal of a forwarded request. The
positive control (`worker-2`) must reach the hermetic echo listener through the
proxy and leave none. Nothing here skips: a missing substrate is a failure.
"""

import argparse
import http.server
import json
import os
from pathlib import Path
import shutil
import sqlite3
import struct
import subprocess
import sys
import tempfile
import threading

ECHO_ADDRESS = ('127.0.0.1', 18081)
ECHO_BODY = b'EPIC17_HERMETIC_ECHO'
EXPECTED_BLOCKS = [
    'sandbox.block.container.escape.fs.control-json',
    'sandbox.block.container.escape.fs.host-home-ssh',
    'sandbox.block.egress.example.invalid:443',
]
# Public sentinels: a value from the operator's environment must never reach a
# Worker or a journal payload.
SENTINELS = {
    'MAOS_OPERATOR_CUSTODY_SENTINEL': 'PUBLIC_OPERATOR_SENTINEL_17_1',
    'CODEX_API_KEY': 'PUBLIC_CODEX_SENTINEL_17_1',
    'ANTHROPIC_API_KEY': 'PUBLIC_CLAUDE_SENTINEL_17_1',
}
KERNEL_ORIGIN = 3


class Failure(Exception):
    pass


def annotate(kind, title, message):
    text = message.replace('%', '%25').replace('\r', '%0D').replace('\n', '%0A')
    print(f'::{kind} title={title}::{text}', flush=True)


class Echo(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        request = self.rfile.read(int(self.headers.get('Content-Length', '0')))
        if self.path != '/echo' or request != ECHO_BODY:
            self.send_error(400)
            return
        self.send_response(200)
        self.send_header('Content-Length', str(len(ECHO_BODY)))
        self.send_header('Connection', 'close')
        self.end_headers()
        self.wfile.write(ECHO_BODY)

    def log_message(self, *_):
        pass


def require_static(binary):
    data = binary.read_bytes()
    if data[:6] != b'\x7fELF\x02\x01' or struct.unpack_from('<H', data, 18)[0] != 62:
        raise Failure(f'{binary} is not an ELF64 x86_64 binary')
    offset = struct.unpack_from('<Q', data, 32)[0]
    size, count = struct.unpack_from('<HH', data, 54)
    if any(struct.unpack_from('<I', data, offset + size * index)[0] == 3 for index in range(count)):
        raise Failure(f'{binary} has a PT_INTERP loader; the T3 probe must be static')


def run(command, environment, log, timeout):
    result = subprocess.run(command, env=environment, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, text=True, timeout=timeout)
    with log.open('a', encoding='utf-8') as output:
        output.write(f'\n$ {" ".join(map(str, command))}\nrc={result.returncode}\n')
        output.write(f'--- stdout\n{result.stdout}--- stderr\n{result.stderr}')
    for stream in (result.stdout, result.stderr):
        if any(line.startswith(('SKIP ', 'SKIP:')) for line in stream.splitlines()):
            raise Failure(f'{command[1]} printed a SKIP line on a declared-capable runner')
    if result.returncode != 0:
        raise Failure(f'{" ".join(map(str, command[1:]))} exited {result.returncode}; see {log.name}')
    return result


def load_pid(database, name):
    rows = database.execute(
        "SELECT spirit_pid, payload_redacted FROM transparency_log WHERE kind = 7 AND intent = 'lifecycle.load'"
    ).fetchall()
    pids = {pid for pid, payload in rows if json.loads(bytes(payload)).get('spirit_id') == name}
    if len(pids) != 1:
        raise Failure(f'expected one lifecycle.load pid for {name}, found {sorted(pids)}')
    return pids.pop()


def verify(database_path, audit_output):
    with sqlite3.connect(f'file:{database_path}?mode=ro', uri=True) as database:
        worker = load_pid(database, 'worker')
        control = load_pid(database, 'worker-2')
        blocks = database.execute(
            'SELECT spirit_pid, intent, origin FROM transparency_log WHERE kind = 8 ORDER BY intent'
        ).fetchall()
        guest = [json.loads(bytes(payload)).get('line', '') for (payload,) in database.execute(
            'SELECT payload_redacted FROM transparency_log WHERE kind = 21 AND spirit_pid = ? ORDER BY rowid',
            (worker,))]
        control_guest = [json.loads(bytes(payload)).get('line', '') for (payload,) in database.execute(
            'SELECT payload_redacted FROM transparency_log WHERE kind = 21 AND spirit_pid = ? ORDER BY rowid',
            (control,))]
        payloads = [bytes(payload) for (payload,) in database.execute(
            'SELECT payload_redacted FROM transparency_log')]
    worker_blocks = [intent for pid, intent, _ in blocks if pid == worker]
    if worker_blocks != EXPECTED_BLOCKS:
        raise Failure(f'worker kind-8 rows {worker_blocks}, expected {EXPECTED_BLOCKS}')
    if any(pid == control for pid, _, _ in blocks):
        raise Failure('the positive control (worker-2) produced a kind-8 row')
    if len(blocks) != 3 or any(origin != KERNEL_ORIGIN for _, _, origin in blocks):
        raise Failure(f'every kind-8 row must be one of the three Kernel-origin rows: {blocks}')
    for line in ('worker-probe-fixture: direct loopback denied with errno 111',
                 'worker-probe-fixture: direct public-network denied with errno 101',
                 'worker-probe-fixture: forwarded forbidden destination returned HTTP 403'):
        if line not in guest:
            raise Failure(f'probing Worker did not report: {line}')
    if 'worker-probe-fixture: forwarded echo returned HTTP 200 with expected body' not in control_guest:
        raise Failure('positive control did not reach the echo listener through the proxy')
    for value in SENTINELS.values():
        if any(value.encode() in payload for payload in payloads):
            raise Failure(f'an operator sentinel reached the Transparency Log: {value}')
    rendered = [line.split() for line in audit_output.splitlines()[1:] if line.strip()]
    if sum(1 for columns in rendered if len(columns) > 3 and columns[3] == 'sandbox.block') != 3:
        raise Failure('`maos audit query --spirit worker --format plain` must list exactly three sandbox.block rows')
    keys = sorted(line.removeprefix('worker-probe-fixture: environment key ')
                  for line in guest if line.startswith('worker-probe-fixture: environment key '))
    return worker, control, keys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--maos', type=Path, required=True)
    parser.add_argument('--probe', type=Path, required=True, help='statically linked worker-probe-fixture')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--image', default=os.environ.get('RUNNER_NAME', 'local'))
    arguments = parser.parse_args()
    arguments.output.mkdir(parents=True, exist_ok=True)
    log = arguments.output / 'line1.log'
    title = f'Epic-17 exit lines 1/1b ({arguments.image})'
    server = None
    try:
        for name in ('MAOS_T3_RUNTIME', 'MAOS_T3_IMAGE_LOCK_PATH', 'MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX'):
            if not os.environ.get(name):
                raise Failure(f'{name} is unset: the signed T3 substrate was not provisioned')
        maos = arguments.maos.resolve()
        require_static(arguments.probe)
        # resolve_cli_binary searches the root binary's directory first.
        shutil.copy2(arguments.probe, maos.parent / 'worker-probe-fixture')
        server = http.server.ThreadingHTTPServer(ECHO_ADDRESS, Echo)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        with tempfile.TemporaryDirectory(prefix='maos-epic-17-exit-') as scratch:
            home = Path(scratch) / 'maos-home'
            grants = Path(scratch) / 'host-grants.toml'
            grants.write_text('[[grant]]\nattested_image = "worker-probe-fixture"\n'
                              'signing_key_id = "MAOS 17-1 Echo"\npermitted_tier = "T3"\n'
                              'permitted_egress_destinations = ["127.0.0.1:18081"]\n')
            environment = dict(os.environ, MAOS_HOME=str(home), MAOS_HOST_GRANTS=str(grants), **SENTINELS)
            # The operator door's bearer file exists on the host; the Worker's
            # open of the same path must still fail inside the container.
            run([maos, 'init'], environment, log, 60)
            if not (home / 'control.json').is_file():
                raise Failure('maos init did not create the host control.json the probe must not reach')
            run([maos, 'run', 'spirits/topologies/worker-sandbox.toml', '--once'], environment, log, 300)
            audit = run([maos, 'audit', 'query', '--spirit', 'worker', '--format', 'plain'], environment, log, 60)
            (arguments.output / 'audit-query-worker.txt').write_text(audit.stdout)
            database = home / 'audit' / 'transparency.sqlite'
            shutil.copy2(database, arguments.output / 'transparency.sqlite')
            worker, control, keys = verify(database, audit.stdout)
        annotate('notice', title, f'worker pid {worker}: 3 Kernel-origin kind-8 rows {EXPECTED_BLOCKS}; '
                 f'direct dials errno 111/101; forwarded forbidden request HTTP 403')
        annotate('notice', title, f'worker-2 pid {control}: echo HTTP 200 through the proxy, 0 kind-8 rows')
        annotate('notice', title, f'guest environment keys: {",".join(keys)}; no operator sentinel journaled')
        return 0
    except (Failure, subprocess.TimeoutExpired, OSError, sqlite3.Error, ValueError) as error:
        annotate('error', title, str(error))
        return 1
    finally:
        if server is not None:
            server.shutdown()
            server.server_close()


if __name__ == '__main__':
    sys.exit(main())
