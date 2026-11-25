# Peer identity rotation — renewing a cohort host's mTLS leaf

**Applies to:** `cohort-a2a-daemon`, `network`-feature builds.
**Decision records:** [ADR-063](../adr/ADR-063-two-axis-revocation-and-post-grace-identity.md) (amendment, Story 23-7); [ADR-062](../adr/ADR-062-mutating-operator-surface.md) (serving-leaf replacement is never an operator POST).
**Story:** `23-7-peer-identity-rotation`.

A cohort host's mTLS leaf comes from **your** PKI. MAOS does not issue it. A rotation is a PKI write plus a signed cohort-manifest reissue; there is no `maosctl` verb and no restart.

## 1. The order of a rotation (host X)

1. **PKI writes the new leaf and key** to X's configured `own_cert_chain` and `own_private_key` paths. The running daemon keeps serving the old leaf; files written early are harmless until X restarts (see the warning in §4).
2. **Sign and push a cohort-manifest reissue** that names the new fingerprint for X (`members[].fingerprint`).
3. **Every peer opens a window.** As each peer applies the reissue it accepts *both* X leaves for
   `G = cold_deployment_t_grace() + confirmation_interval()` of that manifest (5 s + `(t_stale_secs + 1) / 2`; **65 s** at the default `t_stale_secs = 120`, 20 s at the minimum 30, about 30 min at the maximum 3600). `G` is derived, not configured: members apply one reissue up to one pull interval apart.
4. **X swaps its own leaf** as soon as it has applied the reissue and the configured files hold the declared leaf: serving, dialing and the Send-seam team declaration move together. It re-checks on every confirmation tick while its declaration and its served leaf differ.
5. **After `G` each peer promotes** the new leaf, retires the old one and persists it (`promoted = true` in `a2a-pins-<hex(local_host)>.json`). A peer presenting the retired leaf is refused and journaled as `CertPostGraceReject` (below).

Observe it:

- `GET /v1/cohort/self-identity` on X: `serving == declared`, verdict `agree` (or `unconfirmable` until every peer was observed). `diverged` means the files do not hold the declared leaf yet.
- `GET /v1/a2a/rotation-windows` on each peer: the window is `open`, then gone once `G` elapsed.
- The Transparency Log (`maosctl audit query --intent-contains a2a:cert-rotation`): `cert_rotation_window_opened` / `cert_rotation_window_closed` per peer, and on X `self_leaf_swapped`, `self_leaf_swap_pending` (the files do not hold the declared leaf yet; one row per condition) or `self_leaf_refused_undeclared` / `self_leaf_refused_unusable`.

## 2. The PKI profile

| Item | Requirement |
|---|---|
| Subject name | Not used for identity. The pin (SHA-256 of the leaf DER) is the identity. |
| Key usage | One leaf both serves and dials: it needs `serverAuth` **and** `clientAuth` where an extended key usage is present. |
| Trust | `ca_roots` set: the leaf must chain to it, then be pinned. Unset: the leaf is its own anchor, then pinned. |
| SAN (`peer_san = "required"`) | Exactly one URI SAN: `spiffe://<peer_trust_domain>/maos/peer/<peer_id>`. `peer_id` is `[a-zA-Z0-9._-]`. A second URI SAN, a foreign trust domain or no URI SAN is refused. |

`[tcp] peer_san = "off"` is the default: the listen side keeps the flat lookup (any active pin owns a client leaf) and the SAN is ignored. Set `peer_san = "required"` and `peer_trust_domain = "<domain>"` only after **every** host's leaf carries its SAN; the daemon refuses to boot, and refuses every own-leaf swap, with an own leaf that lacks the SAN naming this host. The SAN selects which pin record a client is checked against; the fingerprint still decides.

## 3. Recovery table

| Situation | What happens | What you do |
|---|---|---|
| Reissue signed, files not written yet | X stays on the old leaf; one `self_leaf_swap_pending` row; `self-identity` is `diverged`. Peers that already promoted refuse X (fail-closed partition). | Write the files. The next confirmation tick swaps, with no restart and no new reissue. |
| Files hold a leaf the manifest does not declare | Never served; `self_leaf_refused_undeclared`. | Write the declared leaf, or reissue the manifest for the leaf you meant. |
| Files unreadable, or the leaf lacks the SAN under `required` | Not served; `self_leaf_refused_unusable` and a daemon log line. | Fix the files. |
| X crashes after the files were written, before the swap | X restarts; boot check (c) compares the files to the **on-disk** signed manifest. | Put the reissued manifest at `file.manifest_path` first, then start X. Otherwise boot refuses ("identity on disk is stale"). |
| A peer was down for the whole rotation | On restart it still pins the old leaf and has no promotion: it refuses X (`PeerIdentityUnverified`) and accepts nothing undeclared. | It heals when it pulls the reissue from any member that applied it: a window opens, the new leaf is accepted and promoted after `G`. |
| Two-member cohort (no third member to pull from) | The missed peer can learn the reissue only from X, which it refuses; there is no other push path (the reissue arrives as a `cohort:manifest-reissue` frame over an authenticated peer connection). | Restart it with the new signed manifest at `file.manifest_path` and `[[tcp.peer_pins]]` / `file.peers` naming X's new leaf (boot checks (a) and (b) require them to agree with the manifest). |
| A peer restarts in the middle of a window | The window is process-local and is not restored: the old leaf is the only pin until the next pull reopens it. | Nothing; the next confirmation pull reopens it. |
| A promotion was not carried into config | The next restart refuses: `PinPromotionNotCarried`. | Carry the promoted fingerprint into `[[tcp.peer_pins]]` and the signed on-disk manifest, then restart (or delete the pin file to boot from the configured pin). |
| Key compromise | See §5. | Reissue immediately; the exposure is bounded, not zero. |

## 4. Warning: PKI auto-renewal

Do **not** let a PKI agent (ACME client, cert-manager, a SPIRE agent) rewrite the configured `own_cert_chain` / `own_private_key` paths outside a rotation. The running daemon ignores such a file (the equality guard: the declared fingerprint is the only leaf it will swap to), but the next restart's boot check (c) refuses to boot with an own leaf the on-disk signed manifest does not declare. Renew into a staging path and copy it over the configured paths as step 1 of §1.

## 5. Key compromise: the exposure bound

There is no immediate hard cut. A reissue that moves X to a new leaf retires the compromised leaf on every peer within *the pull lag* (up to one `confirmation_interval()`, until that peer applies the reissue) **plus** `G` (the window). At the default `t_stale_secs = 120` that is about 125 s `[INFERENCE]`. A peer that cannot pull keeps accepting the old leaf until its manifest lease goes stale. Emergency order: issue the new leaf, write it to X's paths, sign and push the reissue, then watch `rotation-windows` close on every peer.

## 6. Refusal reasons

- `CertPostGraceReject` (kind-22 `ConsentRupture`, `rejected[].reason`): a peer presented a leaf that is not the current or open-window pin of a peer whose pin record carries a promotion. It does **not** prove the leaf was the retired one (no retired fingerprint is stored). The tracing token `cert_post_grace_reject` is unchanged.
- `PeerIdentityUnverified`: everything else: an unknown leaf, a peer never promoted, and every `peer_san = "off"` listen-side refusal (the listen side has no subject there).
