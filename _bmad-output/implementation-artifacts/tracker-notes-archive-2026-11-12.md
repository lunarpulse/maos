# Tracker notes archive — 2026-11-12 (rows dropped by operator ruling `drop`)

Rows removed from `sprint-status.yaml` on 2026-11-12 by operator ruling `drop`
(`../planning-artifacts/decisions-2026-11-12-dedrift.md`). Each row is copied verbatim from the tracker at
tree `7ec4b519` (original line number given). None was `done`; none is claimed done. They are not work any
more; where a claim boundary applies it is stated in `RELEASE-HOLDS.md` and the release notes.

## ops-google-calendar-oauth-mcp

Tracker line 433, status `backlog` at removal:

```
ops-google-calendar-oauth-mcp: backlog  # RULED 2026-11-12: needs a real Google account/OAuth: UNCLAIMED, never claimed; fixture server carries the proof; labelled in release notes. Google Calendar MCP with real OAuth for Butler (18-1); CI uses the conformant fixture server only
```

## ops-rpo-rto-cadence-proof

Tracker line 438, status `backlog` at removal:

```
ops-rpo-rto-cadence-proof: backlog  # RULED 2026-11-12: operator-backed production proof, no resources: UNCLAIMED, never claimed; labelled in release notes. production RPO/RTO cadence proof (20-3c D-20-3c-11 B; ask rows203c); operator-backed, never CI
```

## later-bedrock-vertex-and-kms-backends

Tracker line 439, status `backlog` at removal:

```
later-bedrock-vertex-and-kms-backends: backlog  # post-v1.0: Bedrock (SigV4, no aws crate in lock) + Vertex (OAuth JWT) drivers; Vault/cloud-KMS behind KeyManagementPort; was half of 19-5
```

## v25-canary-rollback-and-native-push

Tracker line 445, status `backlog` at removal:

```
v25-canary-rollback-and-native-push: backlog  # v2.5 parking (was half of 14-4): sentinel canary auto-rollback (NFR-Rel-5 ≤30s revert) + native mobile push adapter (§7.4). No journey depends on them.
```
