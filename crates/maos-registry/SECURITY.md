# Spirit Registry Security Posture (v0.5-α)

## Transport Security

The v0.5-α registry server runs **HTTP only (cleartext) on 127.0.0.1:6789**.
Operators who need TLS MUST place an HTTPS-terminating reverse proxy
(nginx / Caddy) in front of the registry server. Direct HTTPS support in
the registry binary is deferred to Story 7.2 (`[registry].tls_cert_path`).

### Recommended reverse-proxy configuration (nginx)

```nginx
server {
    listen 443 ssl;
    server_name registry.example.com;
    ssl_certificate /etc/ssl/registry.crt;
    ssl_certificate_key /etc/ssl/registry.key;
    location /mcp {
        proxy_pass http://127.0.0.1:6789;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

## Authentication

v0.5-α the server is **open** — any client that can reach the TCP port
can invoke any registry operation. Per-operator-key authentication and
per-op ACL are deferred to Story 7.2.

## Per-Publisher Allowlist

v0.5-α the publisher public key for `public-untrusted` Spirits is verified
for Ed25519 signature (self-signed envelope) but NOT cross-checked against
an operator-configured publisher allowlist. The publisher-allowlist feature
is deferred to v0.7 (Story 7.2 prep).

## mTLS for Client-Side Registry Calls

Registry calls at v0.5-α flow through the same `StreamableHttpTransport` as
MCP tool calls. Operator-deployed registries use HTTPS via reverse proxy;
mTLS client-cert authentication is added when bilateral A2A's mTLS
infrastructure (Story 6.3) lands.

## Forward-Shape Contracts

- **Story 7.2**: HTTPS direct in registry binary, operator-key auth, per-op ACL,
  air-gapped import.
- **Story 7.3**: Full ComplianceClaim semantic evaluator (principle engine +
  N=600 corpus) — replaces the v0.5-α structural-only verification.
- **Story 9.4**: Operator surface + air-gapped network-namespace isolation for
  registry-bound traffic.
- **v0.7**: Per-publisher allowlist.
- **v2.5**: `public-vetted` trust tier (FR37).
