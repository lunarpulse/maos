Static scope-selector inventory at HEAD `8e93ffad` — read-only scout `Create1711.ScopeInventory` (2026-10-23), no cargo.
Full transcript: `history://Create1711.ScopeInventory`. The executed regression list is `prototype-A-regressions.md` (this directory);
where the two differ, the executed run wins.

Summary
- Only production producer of declared scopes: `crates/maos-manifest/src/manifest.rs:820-867` (`capabilities_required_to_scopes`),
  stored by `crates/maos-kernel-core/src/security/mod.rs:336-356`. It produces ProviderInfer (prefix before the first `.` of
  `provider.complete[]`, `:823-838`), McpCall (`:839-845`), LoomRead/Write/Scan (`:847-855`), IacSend (`:856-860`, closed set
  `broadcast` / `spirit:peer`, `:625-627`, validated `:769-788`), LogRecall/LogFetch (`:861-866`).
  NOT produced by admission: FsRead, FsWrite, NetHttps, ProcExec, SubSpiritSpawn, MemRead, MemWrite, CliSubprocessSpawn, GatewaySend,
  DistillateWrite, SkillAuthorSelf.
- Scheduler auto-admission builds EMPTY capabilities (`crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:326-343`).
- Smoke branch declares `ProviderInfer("smoke-revocation")` (`crates/maos-bin/src/main.rs:6981-6984`) and requests the same (`:7029-7036`).
- No `"*"`, `"any"` or `""` selector in any admitted grant. PDP representative scopes with empty selectors
  (`crates/maos-pdp/src/reconcile.rs:40-89`) derive action keys only; never granted or minted.
- NetHttps wildcard: none in `spirits/`, `examples/`, `templates/`, `crates/*/tests/fixtures`, inline TOML or JSONL corpora.
  The only inline NetHttps selector is exact `export.arxiv.org` in a schedule fixture (`manifest.rs:4666`), a request, not a grant.
  `*.example.com` at `tests/corpora/calibration-seed-v0.1.jsonl:93` is prose input, not a selector.
- FsRead/FsWrite: no manifest declaration exists (no schema field). Test grants are absolute (`/tmp`, `/tmp/revocation`,
  `/srv/digest`); requests equal the grant. `spirit_session.rs:275-278` adds an FsRead to the launch SANDBOX only (not the policy table).

Requesters (production)
| Caller | Scope | Selector source | Under exact matching |
|---|---|---|---|
| `main.rs:969-980` Butler MCP (pairs `:909-956`) | McpCall | hard-coded pairs | match `spirits/butler/manifest.toml:26-40` |
| `main.rs:1117-1126`, `:1218-1227` Researcher MCP | McpCall | `searches` `:1076-1098`, fetch `:1177-1201` | match `spirits/researcher/manifest.toml:61-75` |
| `main.rs:5667-5674` Researcher inference | ProviderInfer | router default (`:5438-5443` live, `:5467-5470` explicit mode) or `"replay"` (`:5507`) | REFUSED unless the router default equals the declared `anthropic`; `"replay"` always refused |
| `shell_host.rs:91-98` (`main.rs:3701`, `:3842`) hello-spirit turn | ProviderInfer | `router.default_id().unwrap_or("anthropic")` | REFUSED when the router default is not `anthropic` |
| `main.rs:8369-8377` one-shot hello | ProviderInfer | same | same |
| `main.rs:7029-7036` smoke | ProviderInfer | `smoke-revocation` | match |
| `main.rs:9987-9996`, `cross_team_crossing.rs:1050-1130` | Loom* | unit | match |
| `recall_mediation.rs:103-112` (`:132`, `:213`, `:303`) | LogRecall/LogFetch | unit | match |
| `spirit_session.rs:756-765` | IacSend | `broadcast` / `spirit:peer` from `frame.to`; precheck `:866-877` already exact | match |
| `spirit_session.rs:882-893` TaskAssign `task.scope[]` | any | frame-borne, verbatim | refused when the selector is undeclared (intended) |
| `schedule_watchdog.rs:303-324` | `side_effect_scopes[]` | schedule entry, verbatim | refused when undeclared (intended) |
| `worker_spawn.rs:924-932` | CliSubprocessSpawn | resolved path, `aph` (`:863`), manifest version | no policy grant exists → `KernelPolicyDenied` → D1 proceed (`:946-990`); unchanged |

Use-time selector checks: MCP `crates/maos-kernel-core/src/mcp/mod.rs:68-75` (server and tool), inference
`crates/maos-kernel-core/src/inference/mod.rs:240-243` (provider). None for Fs/Net/ProcExec/SubSpirit/Mem/IacSend/Gateway/CLI triplet.
GatewaySend: manifest `[[gateway]].outbound_allowlist` (`manifest.rs:4800-4815`) never becomes a grant; `scope_to_intent` refuses it
(`crates/maos-kernel-core/src/capability/mod.rs:105-108`); dispatcher stub ignores token and recipient
(`crates/maos-kernel-core/src/orchestrator/gateway_dispatcher.rs:127-128`, `:457-463`).

Test-constructed grants: every selector-bearing fixture requests exactly what it declares (list in the transcript §4); no positive test
relies on a same-kind/different-selector grant.
