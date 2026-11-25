# Peer identity rotation — renewing a cohort host's mTLS leaf

**Applies to:** `cohort-a2a-daemon`, `network`-feature builds.
**Decision records:** [ADR-063](../adr/ADR-063-two-axis-revocation-and-post-grace-identity.md) (amendment, Story 23-7); [ADR-062](../adr/ADR-062-mutating-operator-surface.md) (serving-leaf replacement is never an operator POST).
**Story:** `23-7-peer-identity-rotation`.

A cohort host's mTLS leaf comes from **your** PKI. MAOS does not issue it. A rotation is a PKI write plus a signed cohort-manifest reissue; there is no `maosctl` verb and no restart.

## 1. The order of a rotation (host X)

1. **PKI writes the new leaf and key** to X's configured `own_cert_chain` and `own_private_key` paths. The running daemon keeps serving the old leaf; files written early are harmless until X restarts (see the warning in §4).
2. **Sign and push a cohort-manifest reissue** that names the new fingerprint for X (`members[].fingerprint`).
3. **Every peer opens a window.** As each peer applies the reissue it accepts *both* X leaves for
   `G = cold_deployment_t_grace() + 2 × confirmation_interval()` (5 s + `2 × ((t_stale_secs + 1) / 2)`, the larger of the new manifest's and the manifest in force's interval; **125 s** at the default `t_stale_secs = 120`, 35 s at the minimum 30). `G` is derived, not configured: members apply one reissue up to one pull interval apart, and X's own swap trails the last peer's apply by up to one more.
4. **X swaps its own leaf** once it has applied the reissue, the configured files hold the declared leaf (checked for the exact host SAN, the chain and both TLS usages), **and every peer it has an edge with has told it, in a `Pull`, that it holds that version** — a peer that has not applied the reissue pins only the old leaf and would refuse X both ways. The check and the swap are one step under the manifest lock, so a newer reissue cannot slip in between. If a peer stays silent, X swaps after one `t_stale_secs` lease anyway (§3). Serving, dialing and the Send-seam team declaration move together. It re-checks on every confirmation tick and whenever a peer's `Pull` arrives while its declaration and its served leaf differ.
5. **After `G` each peer promotes** the new leaf, retires the old one and persists it (`promoted = true` in `a2a-pins-<hex(local_host)>.json`). A peer presenting a leaf that is not the pin is refused; where the leaf chained to `ca_roots` and the subject was promoted, the refusal is journaled as the candidate signal `CertPostGraceReject` (§6).

Observe it:

- `GET /v1/cohort/self-identity` on X: `serving == declared`, verdict `agree` (or `unconfirmable` until every peer was observed). `diverged` means the files do not hold the declared leaf yet.
- `GET /v1/a2a/rotation-windows` on each peer: the window is `open`, then gone once `G` elapsed.
- The Transparency Log (`maosctl audit query --intent-contains a2a:cert-rotation`): `cert_rotation_window_opened` / `cert_rotation_window_closed` per peer, and on X `self_leaf_swapped`, `self_leaf_swap_pending` (the files do not hold the declared leaf yet; one row per condition) or `self_leaf_refused_undeclared` / `self_leaf_refused_unusable`.
  A swap held for unconfirmed peers is `self_leaf_swap_held`; one that ran after the stale lease with peers still unconfirmed is `self_leaf_swapped_unconfirmed`.

## 2. The PKI profile

| Item | Requirement |
|---|---|
| Subject name | Not used for identity. The pin (SHA-256 of the leaf DER) is the identity. |
| Key usage | One leaf both serves and dials: it needs `serverAuth` **and** `clientAuth` where an extended key usage is present. Checked before the swap, for both usages. |
| Trust and chain | `ca_roots` set: the leaf **and the intermediates in the chain file** must chain to it for both usages, then be pinned; a leaf whose intermediate is not in the file yet is refused. Unset: the leaf is its own anchor (no chain exists to check), then pinned. |
| SAN (`peer_san = "required"`) | Exactly one URI SAN: `spiffe://<peer_trust_domain>/maos/peer/<peer_id>`. `peer_id` is `[a-zA-Z0-9._-]`. A second URI SAN, a foreign trust domain or no URI SAN is refused. |

`[tcp] peer_san = "off"` is the default: the listen side keeps the flat lookup (any active pin owns a client leaf) and the SAN is ignored. Set `peer_san = "required"` and `peer_trust_domain = "<domain>"` only after **every** host's leaf carries its SAN; the daemon refuses to boot, and refuses every own-leaf swap, with an own leaf that lacks the SAN naming this host. The SAN selects which pin record a client is checked against; the fingerprint still decides.

## 3. Recovery table

| Situation | What happens | What you do |
|---|---|---|
| Reissue signed, files not written yet | X stays on the old leaf; one `self_leaf_swap_pending` row; `self-identity` is `diverged`. Peers that already promoted refuse X (fail-closed partition). | Write the files. The next confirmation tick swaps, with no restart and no new reissue. |
| Reissue applied, files written, but a peer has not confirmed the version | X keeps serving the **old** leaf (a peer that has not applied the reissue pins only the old leaf and would refuse X in both directions, and its pulls could not even fetch the reissue from X). One `self_leaf_swap_held` row (its `unconfirmed_peers` field names them). X swaps the moment the last peer's `Pull` declares the version. | Nothing. Make sure the named peers are up and pulling. |
| A peer never confirms (down, partitioned, one-shot courier) | After the manifest's own stale lease (`t_stale_secs`) has elapsed since X applied the reissue, X swaps anyway: that peer is stale by the manifest's own lease. One `self_leaf_swapped_unconfirmed` row names the peers that never confirmed; they refuse X until they apply the reissue (and, if their window has already closed, until they are restarted with the new manifest). | Bring those peers up and let them pull, or restart them with the new signed manifest on disk. |
| The chain file holds the leaf but not its intermediate yet (or the leaf lacks `serverAuth` or `clientAuth`) | Not served; `self_leaf_refused_unusable`, retried on every tick, so the completed PEM swaps by itself. | Write the complete chain. |
| Files hold a leaf the manifest does not declare | Never served; `self_leaf_refused_undeclared`. | Write the declared leaf, or reissue the manifest for the leaf you meant. |
| Files unreadable, or the leaf lacks the SAN under `required` | Not served; `self_leaf_refused_unusable` and a daemon log line. | Fix the files. |
| X crashes after the files were written, before the swap | X restarts; boot check (c) compares the files to the **on-disk** signed manifest. | Put the reissued manifest at `file.manifest_path` first, then start X. Otherwise boot refuses ("identity on disk is stale"). |
| A peer was down for the whole rotation | On restart it still pins the old leaf and has no promotion: it refuses X (`PeerIdentityUnverified`) and accepts nothing undeclared. | It heals when it pulls the reissue from any member that applied it: a window opens, the new leaf is accepted and promoted after `G`. |
| Two-member cohort (no third member to pull from) | The missed peer can learn the reissue only from X, which it refuses; there is no other push path (the reissue arrives as a `cohort:manifest-reissue` frame over an authenticated peer connection). | Restart it with the new signed manifest at `file.manifest_path` and `[[tcp.peer_pins]]` / `file.peers` naming X's new leaf (boot checks (a) and (b) require them to agree with the manifest). |
| A peer restarts in the middle of a window | The window is process-local and is not restored: the old leaf is the only pin until the next pull reopens it. | Nothing; the next confirmation pull reopens it. |
| A promotion was not carried into config | The next restart refuses: `PinPromotionNotCarried`. | Carry the promoted fingerprint into `[[tcp.peer_pins]]` and the signed on-disk manifest, then restart. Deleting the pin file also boots from the configured pin, **but it is not a neutral reset**: it drops the durable manifest-version floor to 0 (an older signed reissue is accepted again, which can re-seed a retired pin) and clears every `promoted` marker (a retired leaf is journaled `PeerIdentityUnverified` instead of `CertPostGraceReject` until the next promotion). Treat it as a last resort and re-apply the newest signed manifest afterwards. |
| Key compromise | See §5. | Reissue immediately; the exposure is bounded, not zero. |

## 4. Warning: PKI auto-renewal

Do **not** let a PKI agent (ACME client, cert-manager, a SPIRE agent) rewrite the configured `own_cert_chain` / `own_private_key` paths outside a rotation. The running daemon ignores such a file (the equality guard: the declared fingerprint is the only leaf it will swap to), but the next restart's boot check (c) refuses to boot with an own leaf the on-disk signed manifest does not declare. Renew into a staging path and copy it over the configured paths as step 1 of §1.

## 5. Key compromise: the exposure bound

There is no immediate hard cut, and the bound below is conditional. A peer retires the compromised leaf `G` after **it applies** the reissue, and it applies the reissue when it pulls it (up to one `confirmation_interval()` per hop, so one interval for a peer that pulls straight from an honest member, one more per relaying hop) — at the default `t_stale_secs = 120` that is about 60 s + 125 s = 185 s for a direct puller `[INFERENCE]`. A restart in the middle of a window adds one more pull lag plus `G` (the window is process-local). The conditions: **(1)** each peer has an honest pull path to some member that holds the new version. An attacker holding the old key who sits on a peer's only pull path to X can keep that peer fresh by replaying the version it already holds (an equal-version confirmation refreshes the lease), so that peer stays on the old leaf indefinitely; the lease does not bound this. **(2)** `G` and the swap gate are sized from the larger of the new manifest's and the manifest in force's `confirmation_interval()`, so a reissue that rotates a leaf and lowers `t_stale_secs` does not shrink the cover. Emergency order: issue the new leaf, write it to X's paths, sign and push the reissue to every member by a path the attacker does not control, then watch `rotation-windows` close on every peer. A peer you cannot reach must be restarted with the new signed manifest on disk.

## 6. Refusal reasons

- `CertPostGraceReject` (kind-22 `ConsentRupture`, `rejected[].reason`; row intent `a2a:peer-identity-refusal`, `to_spirit_id` = `<direction>@<peer>`): a **candidate** retired-key signal, not proof. It is typed only when the presented leaf chained to your `ca_roots`, did not match the subject peer's current or open-window pin, and that peer's pin record carries a promotion. No retired fingerprint is stored, so any other CA-issued leaf carrying that peer's SAN reads the same. `<peer>` is the dial target or, on the listen side under `required`, the SAN-named peer; with no parsed SAN it is the socket address, so the rotation-close ⋈ refusal join cannot be made from durable rows for those refusals. The tracing token `cert_post_grace_reject` is unchanged.
- `PeerIdentityUnverified`: everything else: an unknown or non-chaining leaf, a peer never promoted, a promoted peer that **restarted** and presents its current leaf (its pin is invalidated until re-pin approval; approve it, do not read the row as a retired key), every handshake with `ca_roots` **unset** (anyone can present a self-signed leaf naming a promoted peer, so no retired-key signal is made), and every `peer_san = "off"` listen-side refusal (the listen side has no subject there).
