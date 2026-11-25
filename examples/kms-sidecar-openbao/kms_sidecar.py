#!/usr/bin/env python3
"""maos.kms.v1 sidecar for OpenBao Transit (Story 23-8 reference example).

NOT a release artifact and NOT security-reviewed. It serves the MAOS KMS wire
(docs/reference/kms-wire-v1.md) on a loopback address and translates it to
OpenBao Transit encrypt/decrypt (OpenBao v2.7.3, MPL-2.0). Python 3 standard
library only; nothing here is part of the Cargo workspace.

    POST /wrap    {"version","plaintext":<b64 32 bytes>}  -> {"version","wrapped":<b64>}
    POST /unwrap  {"version","wrapped":<b64>}             -> {"version","plaintext":<b64>}
    GET  /health                                          -> {"version","status":"ok"}
    typed refusal of one payload: 422 {"error":"wrap_failed"|"unwrap_failed"}
    anything else that is not an answer (upstream down, bad token): 502 / 503

Environment (the OpenBao token is read from the environment, never from a
file or argument, and is never logged):

    BAO_TOKEN              required. Sent to OpenBao as the X-Vault-Token header.
    BAO_ADDR               OpenBao address, default http://127.0.0.1:8200
    BAO_TRANSIT_KEY        Transit key name, default "maos" (aes256-gcm96)
    BAO_TRANSIT_MOUNT      Transit mount path, default "transit"
    SIDECAR_BIND           host:port to serve on, default 127.0.0.1:8300; a
                           non-loopback host is refused unless
                           SIDECAR_ALLOW_NON_LOOPBACK=1 (then put TLS in front)
    SIDECAR_BEARER         optional. When set, wrap and unwrap require
                           "Authorization: Bearer <value>"; set MAOS_KMS_BEARER
                           to the same value on the MAOS side.

The call to OpenBao never follows a redirect (the token would travel to the
redirect target; a 3xx is an upstream failure) and ignores HTTP_PROXY /
HTTPS_PROXY / ALL_PROXY (the token and the data key would pass through the
proxy). Each client connection has a socket timeout, so a client that stalls
cannot hold a handler thread.
"""

import base64
import binascii
import hmac
import http.server
import ipaddress
import json
import os
import sys
import urllib.error
import urllib.request

VERSION = "maos.kms.v1"
MAX_BODY = 16 * 1024
MAX_WRAPPED = 4096
UPSTREAM_TIMEOUT = 3
HANDLER_TIMEOUT = 10


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None  # the 3xx surfaces as an HTTPError: an upstream failure


def opener():
    """An opener with no redirect handling and no proxy, built per call so a
    proxy variable set later is still ignored."""
    return urllib.request.build_opener(urllib.request.ProxyHandler({}), _NoRedirect())


class Config:
    def __init__(self, token, addr, key, mount, bearer=None):
        self.token = token
        self.addr = addr.rstrip("/")
        self.key = key
        self.mount = mount.strip("/")
        self.bearer = bearer

    @classmethod
    def from_environment(cls, env=os.environ):
        token = env.get("BAO_TOKEN", "")
        if not token:
            raise SystemExit("BAO_TOKEN is required (the OpenBao token, from the environment)")
        return cls(
            token,
            env.get("BAO_ADDR", "http://127.0.0.1:8200"),
            env.get("BAO_TRANSIT_KEY", "maos"),
            env.get("BAO_TRANSIT_MOUNT", "transit"),
            env.get("SIDECAR_BEARER") or None,
        )

    def __repr__(self):
        return "Config(addr=%r, key=%r, mount=%r)" % (self.addr, self.key, self.mount)


class Upstream(Exception):
    """OpenBao did not give an answer about this payload."""


class Refused(Exception):
    """OpenBao refused this one payload (a typed 422 for MAOS)."""


def transit(config, operation, payload):
    """POST one Transit request; returns the `data` object."""
    url = "%s/v1/%s/%s/%s" % (config.addr, config.mount, operation, config.key)
    request = urllib.request.Request(
        url,
        data=json.dumps(payload).encode(),
        headers={"X-Vault-Token": config.token, "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with opener().open(request, timeout=UPSTREAM_TIMEOUT) as response:
            return json.load(response)["data"]
    except urllib.error.HTTPError as error:
        # 400 is OpenBao saying that this ciphertext or plaintext is invalid.
        if error.code == 400:
            raise Refused() from None
        raise Upstream() from None
    except (urllib.error.URLError, OSError, ValueError, KeyError):
        raise Upstream() from None


def transit_ready(config):
    request = urllib.request.Request(
        "%s/v1/%s/keys/%s" % (config.addr, config.mount, config.key),
        headers={"X-Vault-Token": config.token},
    )
    try:
        with opener().open(request, timeout=UPSTREAM_TIMEOUT) as response:
            return response.status == 200
    except (urllib.error.URLError, OSError):
        return False


def decode_b64(value):
    if not isinstance(value, str):
        raise ValueError("not a string")
    return base64.b64decode(value, validate=True)


class Handler(http.server.BaseHTTPRequestHandler):
    config = None  # set by make_server
    server_version = "maos-kms-sidecar"
    protocol_version = "HTTP/1.1"
    timeout = HANDLER_TIMEOUT  # socket timeout of each client connection

    def log_message(self, fmt, *args):  # method and path only
        sys.stderr.write(
            "sidecar: %s %s\n" % (getattr(self, "command", "-"), getattr(self, "path", "-"))
        )

    def reply(self, status, body):
        data = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(data)

    def authorized(self):
        if self.config.bearer is None:
            return True
        presented = self.headers.get("Authorization", "")
        return hmac.compare_digest(presented.encode(), ("Bearer " + self.config.bearer).encode())

    def read_json(self):
        length = int(self.headers.get("Content-Length", "0"))
        if length <= 0 or length > MAX_BODY:
            raise ValueError("body size")
        body = json.loads(self.rfile.read(length))
        if not isinstance(body, dict) or body.get("version") != VERSION:
            raise ValueError("version")
        return body

    def do_GET(self):
        if self.path != "/health":
            return self.reply(404, {"error": "not_found"})
        if transit_ready(self.config):
            return self.reply(200, {"version": VERSION, "status": "ok"})
        return self.reply(503, {"version": VERSION, "status": "unavailable"})

    def do_POST(self):
        if self.path not in ("/wrap", "/unwrap"):
            return self.reply(404, {"error": "not_found"})
        if not self.authorized():
            return self.reply(401, {"error": "unauthorized"})
        try:
            body = self.read_json()
        except (ValueError, json.JSONDecodeError):
            return self.reply(400, {"error": "bad_request"})
        try:
            if self.path == "/wrap":
                return self.wrap(body)
            return self.unwrap(body)
        except Upstream:
            return self.reply(502, {"error": "upstream"})

    def wrap(self, body):
        try:
            plaintext = decode_b64(body.get("plaintext"))
        except (ValueError, binascii.Error):
            return self.reply(400, {"error": "bad_request"})
        if len(plaintext) != 32:
            return self.reply(400, {"error": "bad_request"})
        try:
            data = transit(self.config, "encrypt", {"plaintext": base64.b64encode(plaintext).decode()})
            ciphertext = data["ciphertext"].encode()
        except Refused:
            return self.reply(422, {"error": "wrap_failed"})
        except (KeyError, AttributeError):
            raise Upstream() from None
        if len(ciphertext) > MAX_WRAPPED:
            raise Upstream()
        return self.reply(200, {"version": VERSION, "wrapped": base64.b64encode(ciphertext).decode()})

    def unwrap(self, body):
        try:
            ciphertext = decode_b64(body.get("wrapped"))
            text = ciphertext.decode("ascii")
        except (ValueError, binascii.Error, UnicodeDecodeError):
            return self.reply(422, {"error": "unwrap_failed"})
        if not text.startswith("vault:v"):
            return self.reply(422, {"error": "unwrap_failed"})
        try:
            data = transit(self.config, "decrypt", {"ciphertext": text})
            plaintext = decode_b64(data["plaintext"])
        except Refused:
            return self.reply(422, {"error": "unwrap_failed"})
        except (KeyError, ValueError, binascii.Error):
            raise Upstream() from None
        if len(plaintext) != 32:
            return self.reply(422, {"error": "unwrap_failed"})
        return self.reply(200, {"version": VERSION, "plaintext": base64.b64encode(plaintext).decode()})


def make_server(config, host="127.0.0.1", port=8300, timeout=HANDLER_TIMEOUT):
    handler = type("BoundHandler", (Handler,), {"config": config, "timeout": timeout})
    return http.server.ThreadingHTTPServer((host, port), handler)


def parse_bind(value, env):
    host, _, port = value.rpartition(":")
    host = host.strip("[]")
    if not ipaddress.ip_address(host).is_loopback and env.get("SIDECAR_ALLOW_NON_LOOPBACK") != "1":
        raise SystemExit("refusing a non-loopback SIDECAR_BIND (set SIDECAR_ALLOW_NON_LOOPBACK=1 and put TLS in front)")
    return host, int(port)


def main():
    config = Config.from_environment()
    host, port = parse_bind(os.environ.get("SIDECAR_BIND", "127.0.0.1:8300"), os.environ)
    server = make_server(config, host, port)
    sys.stderr.write("sidecar: serving maos.kms.v1 on %s:%d for %r\n" % (host, port, config))
    server.serve_forever()


if __name__ == "__main__":
    main()
