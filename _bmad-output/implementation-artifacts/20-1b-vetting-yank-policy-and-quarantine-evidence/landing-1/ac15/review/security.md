# Security review: 20-1b AC15/F5 + cohort teardown (commit f9921251)

Saved by the orchestrator from the security-reviewer's final message (no write tool). Read-only review; counts: 1 Medium, 5 Low, 5 Info; 17 rejected candidates.

- F1 MEDIUM: unauthenticated connection-slot exhaustion: server.rs:17 MAX_CONNECTIONS=8, :20 REQUEST_DEADLINE=5s, accept loop :181-200: 8 idle TCP connections hold all slots (TLS mode drops silently). Fix: per-source-IP cap, shorter handshake/first-byte timeout, separate pre-handshake pool, explicit 503.
- F2 LOW: 64 MiB body allocated per slot before auth (server.rs:384-391). Fix: per-endpoint caps, growing buffer.
- F3 LOW: accept-loop error busy-loop (server.rs:218-220) and fatal per-connection try_clone (~:201). Fix: backoff; drop the connection not the server.
- F4 LOW: client has no total request deadline (http.rs:95-99, :257): drip-feed stalls synchronous admission paths. Fix: absolute per-exchange deadline.
- F5 LOW: no monotonic freshness: verify_head (security.rs) has no comparison to last-seen head; yank.rs apply_list silently passes head < last_seen_ns; cursor file unauthenticated. Fix: head < last_seen_ns = rollback -> resync from 0 + error; persist last verified head; cursor ahead of the signed head = corruption.
- F6 LOW: yanks_since loads and returns the full list per poll (handlers/yanks_since.rs). Fix: lower-bound query, pagination/cap.
- F7 INFO: trust roots = bundled webpki-roots plus an additive operator CA (security.rs:273-279) while the ruling says 'platform WebPKI'; the response signature is the real trust anchor. Fix wording or use platform roots.
- F8 INFO: TLS 1.2 permitted both ends (security.rs:283,:301). Optional TLS1.3-only.
- F9 INFO: several `?` early returns between main.rs:3246 and :4538 drop the poller without join (PollTask::drop aborts); not a new audit-loss route (pre-existing); AST guard cannot see them. Optional.
- F10 INFO: detached poll worker thread at shutdown is intentional.
- F11 INFO: one registry key signs response digests and tier attestations (domain-separated; no exploit).

Rejected: env redirect of the trust root, one-sided dev flag, cross-URI replay, missing-signature fail-open, redirect downgrade, response replay, yank replay, unauthenticated deprecate, publisher impersonation, keyringless vetter yank, key TOCTOU, TLS resumption, IPv4-mapped loopback, trailing-dot host, non-canonical URI, envelope malleability, teardown hang, audit-sender ordering, env override of org key/tier floor.
Not reviewed: handlers/publish.rs, handlers/manifest.rs, maos-compliance vetting/*, bin/server.rs, lib.rs, storage.rs outside yank path, dependency advisories.
