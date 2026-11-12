## Verdict

**overall_correctness: incorrect — acceptance/completion records, not kernel behavior.**  
The production changes match the granted KA + KIO1 + KIO2 shapes; no patch-introduced runtime defect was established. Commit `251dc692` nevertheless marks the story complete while explicitly recording required completion evidence as outstanding.

**confidence: 0.96**

Counts: **P0 0 · P1 0 · P2 1 · P3 0**. No changes or verification commands were executed.

## Finding

### P2 — Keep the story open until its completion requirements are satisfied

- **file_path:** `_bmad-output/implementation-artifacts/20-4-gemini-driver-and-endpoint-pin.md`
- **line_start / line_end:** **23–23**
- **confidence:** **0.99**

At the reviewed commit, `Status: done`, the checked T7/T10 tasks, and the tracker’s `done` value contradict the landing record: no hosted discipline run exists on the L2 final tree, prescribed `prepush` was skipped, and L2 review remained unexecuted (`story:449–452`; `landing-2/command-results.md:24`). The story incorporates the final-tree proof contract (`docs/testing/proof-contract.md:12–17,146–148`), and AC9.2 additionally requires commit-qualified deferred closures, whereas the committed closures still say “commit hash to be recorded by Main” (`deferred-work.md:1401–1403`). The trigger is treating this landing as completed; its impact is an unsupported acceptance/completion state, despite valid local kernel witnesses. Keep completion open until the required evidence or an explicit operator exception is recorded, finalize the closure references to `@251dc692`, and record the closing-tree ratchet result. No additional kernel mutation witness is needed.

**Current-checkout qualification:** the story header already retracts `done` to `in-progress` and names these outstanding items. That addresses the immediate status symptom; this finding describes the requested commit, not a request to repeat that edit.

## Requested checks

### 1. KA — provider identities and buckets

**Pass by code inspection.**

- The changed kernel mapping explicitly interns both lowercase canonical IDs (`inference/mod.rs:96–102`).
- Router dispatch uses exact map lookup, not case folding or aliases (`inference/router.rs:31–40`). Composition-root registration, replay registration, provider catalog, and manifest allowlist consistently use `gemini`/`kimi`.
- Uppercase IDs and aliases receive no special treatment. This preserves the existing exact-ID convention; no acceptance criterion grants aliases.
- Unknown IDs still return `Unconfigured` before driver execution when the limiter is installed. The `bedrock` witness verifies that refusal and zero driver calls.
- `BucketKey` hashes/equates **both provider ID and credential fingerprint**. Gemini and Kimi therefore have **separate buckets even with identical credential bytes** (`rate_limit.rs:30–43`).
- Neither has an explicit quota row; both use the specified **1000 RPM** fallback (`rate_limit.rs:314–319`).
- Consuming dispatch was inspected: router lookup accepts both registered IDs; rate-limit frame delivery has explicit `RateLimited` routing and mailbox enrollment. No new-ID silent discard was found.

### 2. KIO1 — BadHeader redaction

**Pass for the granted BadHeader failure class.**

Both POST APIs reach `http_post_exchange`; its guarded transport arm discards the entire ureq error and constructs a fresh credential-free message (`io/mod.rs:107,131–135`). Consequently, the returned `IoError` retains neither the original error nor its header line.

`IoError`’s Display and derived Debug can expose only that replacement string. It has **no serde implementation** (`maos-domain/src/ports/io_subsystem.rs:66–79`).

Locked ureq 2.12.1 validates request headers at the start of `Request::do_call`, before connection/prelude logging (`request.rs:114–117`). Thus the malformed-header witness does not leak its rejected value through that logger before kernel redaction. The kernel journal records method and URL, not request headers.

**Boundary:** this does not establish blanket secrecy for valid credentials in dependency debug logs; see rejected candidates below.

### 3. KIO2 — status retention and retry classification

**Pass within the specified cap contract.**

- The status-derived truncation flag covers every non-2xx response reaching the exchange reader, independent of a particular status value (`io/mod.rs:128,138–145`).
- Successful body reads exceeding the cap are truncated to exactly 10 MiB; status and requested response header remain attached.
- Oversized 2xx POST and GET responses still produce `Decode`.
- Gemini consumes status before parsing a success response and returns `ProviderRejected { status, … }`. Kernel mapping recognizes **429 → RateLimited**; other statuses remain `ProviderRejected`.
- Router retry classification remains **Transport, 429, and 500–599 retriable**; other rejection statuses are not (`inference/router.rs:74–80`).
- MCP consumes preserved 401 as `Unauthorized`, rather than transport failure (`streamable_http.rs:103–119`).

Actual body-read/decompression failures can still return `Decode`; the port contract explicitly permits that. Legacy `http_post` still flattens non-2xx into `Transport`, as the story explicitly excludes changing existing-driver status surfacing.

### 4. Witnesses and plants

**The recorded substitutions exercise the intended failure classes.**

| Plant | Raw observed failure | Isolation |
|---|---|---|
| Drop Gemini arm | `Unconfigured`, provider witness line 181; parity witness line 218 | IO witnesses remain green |
| Remove BadHeader arm | Credential echoed, IO witness line 67 | Status witness remains green |
| Pass `false` for POST truncation | Oversized 401 becomes `Decode`, IO witness line 90 | Redaction witness remains green |

Each plant log records kernel recompilation and assertion failure—not merely compilation failure. Plant 3 is a valid behavioral reversal of KIO2 even though it leaves the helper’s new signature/branch present.

Recorded evidence: red-before **4 failures / 1 pass**, restored new binaries **5/5**, related focused binaries **15/15**, kernel lib **253 passed**. These are **existing logs inspected**, not reviewer executions.

The witnesses correctly call the real adapter. They do not independently test every serialization/log surface or oversized 429 end-to-end; those paths were inspected rather than falsely claimed exercised.

### 5. Accounting

**Claims reconcile.**

- Inference: **+2 physical / +2 code**.
- IO: **15 additions − 4 deletions = +11 physical**, including four net comment lines: **+7 code**.
- Total production kernel delta: **+13 physical / +9 code**.
- Recorded pins: **26841 → 26854 physical**, **21042 → 21051 tokei**.
- Both modified content digests and the set hash change together; file count remains **95**.
- Clean recorded aggregate **202121**, hardfail **202122**, matches measured-plus-one.

The production diff matches the granted creation prototype.

### 6. Scope and AC6/AC9

No production kernel edit falls outside R. AC6 admission/pin enforcement is unchanged; L1 was not re-reviewed.

Tests, pins, docs, and evidence accompany the landing as expected. One additional **comment-only** change in `xtask/kloc.toml:339` repairs the 20-5 HISTORY citation; the commit explicitly discloses it. It changes neither its numeric ceiling nor production behavior.

AC9’s coverage/docs updates identify the required witnesses and retain the seed-versus-live distinction. Its completion evidence remains subject to the finding above.

## Rejected candidates

- **Status lost on broken/truncated wire bodies:** pre-existing permitted decode-failure behavior, not introduced by KIO2.
- **Kimi/older drivers flatten HTTP statuses:** deliberate, explicitly excluded story scope.
- **Missing case-insensitive IDs or aliases:** unsupported by the exact-ID contract.
- **Valid `x-goog-api-key`/`x-api-key` exposed by ureq debug prelude logging:** ureq only marks Authorization/Cookie sensitive; this is a pre-existing path, not introduced by L2, and is distinct from the fixed BadHeader path.
- **Missing final closing-ratchet green:** existing logs contain `MissingStory` failures before the final HISTORY citation correction. Those failures are not themselves a newly introduced kernel defect; a closing-tree result is still unverified.

## Not verified

No builds, tests, plants, gates, hosted runs, or live vendor calls were executed. Fresh digests/tokei measurements were not recomputed. Hosted portability and final-tree discipline/prepush results remain unverified; operator-owned untracked paths were not inspected.