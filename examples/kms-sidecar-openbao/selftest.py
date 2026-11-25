#!/usr/bin/env python3
"""Self-test of the maos.kms.v1 sidecar against a stub OpenBao Transit.

No OpenBao, no network beyond loopback, no sleeps. Run: python3 selftest.py
"""

import base64
import http.server
import io
import json
import os
import socket
import sys
import threading
import urllib.error
import urllib.request

import kms_sidecar

TOKEN = "bao-token-canary-6c1e"
BEARER = "sidecar-bearer-canary-77aa"
VERSION = "maos.kms.v1"


class StubTransit(http.server.BaseHTTPRequestHandler):
    """encrypt/decrypt with a reversible, non-secret transform; checks the token."""

    mode = "ok"  # "ok" | "down" | "redirect"
    trap_url = None

    def log_message(self, *args):
        pass

    def reply(self, status, body):
        data = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if StubTransit.mode == "redirect":
            return self.redirect()
        if self.headers.get("X-Vault-Token") != TOKEN or StubTransit.mode == "down":
            return self.reply(403 if StubTransit.mode != "down" else 503, {})
        return self.reply(200, {"data": {"name": "maos"}})

    def redirect(self):
        self.send_response(307)
        self.send_header("Location", StubTransit.trap_url + self.path)
        self.send_header("Content-Length", "0")
        self.send_header("Connection", "close")
        self.end_headers()

    def do_POST(self):
        if StubTransit.mode == "redirect":
            if self.headers.get("Content-Length"):
                self.rfile.read(int(self.headers["Content-Length"]))
            return self.redirect()
        if self.headers.get("X-Vault-Token") != TOKEN:
            return self.reply(403, {"errors": ["permission denied"]})
        if StubTransit.mode == "down":
            return self.reply(503, {})
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        if self.path.endswith("/encrypt/maos"):
            raw = base64.b64decode(body["plaintext"])
            return self.reply(200, {"data": {"ciphertext": "vault:v1:" + base64.b64encode(raw[::-1]).decode()}})
        if self.path.endswith("/decrypt/maos"):
            if not body["ciphertext"].startswith("vault:v1:"):
                return self.reply(400, {"errors": ["invalid ciphertext"]})
            try:
                raw = base64.b64decode(body["ciphertext"][len("vault:v1:"):], validate=True)
            except Exception:
                return self.reply(400, {"errors": ["invalid ciphertext"]})
            return self.reply(200, {"data": {"plaintext": base64.b64encode(raw[::-1]).decode()}})
        return self.reply(404, {})


class Trap(http.server.BaseHTTPRequestHandler):
    """A redirect target or proxy: records every request it receives."""

    requests = []

    def log_message(self, *args):
        pass

    def handle_any(self):
        length = int(self.headers.get("Content-Length") or 0)
        if length:
            self.rfile.read(length)
        Trap.requests.append((self.command, self.path, self.headers.get("X-Vault-Token")))
        data = b'{"data":{}}'
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(data)

    do_GET = do_POST = handle_any


# The self-test's own client: it must not be steered by a proxy variable either.
CALL_OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))

def serve(server):
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return thread


def call(base, method, path, body=None, bearer=None, raw=None):
    data = raw if raw is not None else (json.dumps(body).encode() if body is not None else None)
    request = urllib.request.Request(base + path, data=data, method=method)
    if bearer:
        request.add_header("Authorization", "Bearer " + bearer)
    try:
        with CALL_OPENER.open(request, timeout=5) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        return error.code, json.loads(error.read() or b"{}")


def b64(raw):
    return base64.b64encode(raw).decode()


def main():
    transit = http.server.ThreadingHTTPServer(("127.0.0.1", 0), StubTransit)
    serve(transit)
    bao = "http://127.0.0.1:%d" % transit.server_address[1]
    trap = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Trap)
    serve(trap)
    trap_base = "http://127.0.0.1:%d" % trap.server_address[1]

    log = io.StringIO()
    real_stderr, sys.stderr = sys.stderr, log
    try:
        config = kms_sidecar.Config(TOKEN, bao, "maos", "transit", BEARER)
        sidecar = kms_sidecar.make_server(config, "127.0.0.1", 0)
        serve(sidecar)
        base = "http://127.0.0.1:%d" % sidecar.server_address[1]

        key = bytes(range(32))
        # /health
        assert call(base, "GET", "/health") == (200, {"version": VERSION, "status": "ok"})
        # round trip
        status, wrapped = call(base, "POST", "/wrap", {"version": VERSION, "plaintext": b64(key)}, BEARER)
        assert status == 200 and wrapped["version"] == VERSION, (status, wrapped)
        assert base64.b64decode(wrapped["wrapped"]).startswith(b"vault:v1:")
        status, opened = call(base, "POST", "/unwrap", {"version": VERSION, "wrapped": wrapped["wrapped"]}, BEARER)
        assert (status, opened) == (200, {"version": VERSION, "plaintext": b64(key)}), (status, opened)
        # the bearer is enforced on wrap and unwrap
        assert call(base, "POST", "/wrap", {"version": VERSION, "plaintext": b64(key)})[0] == 401
        assert call(base, "POST", "/unwrap", {"version": VERSION, "wrapped": wrapped["wrapped"]}, "wrong")[0] == 401
        # malformed requests are 400
        assert call(base, "POST", "/wrap", {"version": "maos.kms.v0", "plaintext": b64(key)}, BEARER)[0] == 400
        assert call(base, "POST", "/wrap", {"version": VERSION}, BEARER)[0] == 400
        assert call(base, "POST", "/wrap", {"version": VERSION, "plaintext": "!!"}, BEARER)[0] == 400
        assert call(base, "POST", "/wrap", {"version": VERSION, "plaintext": b64(b"short")}, BEARER)[0] == 400
        assert call(base, "POST", "/wrap", raw=b"not json", bearer=BEARER)[0] == 400
        # a foreign wrapped key is a typed 422, not an availability failure
        for foreign in (b64(b"vault:v1:!!!not-base64"), b64(b"garbage"), "!!"):
            status, body = call(base, "POST", "/unwrap", {"version": VERSION, "wrapped": foreign}, BEARER)
            assert (status, body) == (422, {"error": "unwrap_failed"}), (status, body)
        # OpenBao down: 502 on wrap/unwrap, 503 on health
        StubTransit.mode = "down"
        assert call(base, "POST", "/wrap", {"version": VERSION, "plaintext": b64(key)}, BEARER)[0] == 502
        assert call(base, "GET", "/health")[0] == 503
        StubTransit.mode = "ok"
        # a wrong OpenBao token is never a typed refusal of the payload
        wrong = kms_sidecar.make_server(
            kms_sidecar.Config("wrong-token", bao, "maos", "transit"), "127.0.0.1", 0
        )
        serve(wrong)
        wrong_base = "http://127.0.0.1:%d" % wrong.server_address[1]
        assert call(wrong_base, "POST", "/wrap", {"version": VERSION, "plaintext": b64(key)})[0] == 502
        assert call(wrong_base, "GET", "/health")[0] == 503
        # a redirect from OpenBao is never followed: the token must not reach its target
        StubTransit.mode, StubTransit.trap_url = "redirect", trap_base
        assert call(base, "POST", "/wrap", {"version": VERSION, "plaintext": b64(key)}, BEARER)[0] == 502
        assert call(base, "POST", "/unwrap", {"version": VERSION, "wrapped": wrapped["wrapped"]}, BEARER)[0] == 502
        assert call(base, "GET", "/health")[0] == 503
        StubTransit.mode = "ok"
        assert Trap.requests == [], "a redirect target received a request: %r" % (Trap.requests,)
        # a proxy environment variable is ignored: the token and key never pass a proxy
        proxy_vars = ("http_proxy", "HTTP_PROXY", "all_proxy", "ALL_PROXY")
        saved = {name: os.environ.get(name) for name in proxy_vars}
        try:
            for name in proxy_vars:
                os.environ[name] = trap_base
            assert call(base, "POST", "/wrap", {"version": VERSION, "plaintext": b64(key)}, BEARER)[0] == 200
            assert call(base, "GET", "/health")[0] == 200
        finally:
            for name, value in saved.items():
                if value is None:
                    os.environ.pop(name, None)
                else:
                    os.environ[name] = value
        assert Trap.requests == [], "a proxy received a request: %r" % (Trap.requests,)
        # a client that stalls mid-request is released by the socket timeout
        stall = kms_sidecar.make_server(
            kms_sidecar.Config(TOKEN, bao, "maos", "transit"), "127.0.0.1", 0, timeout=0.3
        )
        serve(stall)
        assert kms_sidecar.Handler.timeout and kms_sidecar.Handler.timeout > 0
        with socket.create_connection(stall.server_address, timeout=10) as stalled:
            stalled.sendall(b"POST /wrap HTTP/1.1\r\nContent-Length: 50\r\n\r\n")
            try:
                assert stalled.recv(4096) == b"", "the stalled handler must close the connection"
            except ConnectionResetError:
                pass
        # a non-loopback bind is refused without the explicit opt-in
        try:
            kms_sidecar.parse_bind("0.0.0.0:8300", {})
            raise AssertionError("a non-loopback bind must be refused")
        except SystemExit:
            pass
        assert kms_sidecar.parse_bind("[::1]:8300", {}) == ("::1", 8300)
        assert TOKEN not in repr(config) and BEARER not in repr(config)
    finally:
        sys.stderr = real_stderr
    captured = log.getvalue()
    assert TOKEN not in captured and BEARER not in captured, "a secret reached the log"
    assert "plaintext" not in captured and "wrapped" not in captured, "a body reached the log"
    print("kms-sidecar-openbao self-test: ok")


if __name__ == "__main__":
    main()
