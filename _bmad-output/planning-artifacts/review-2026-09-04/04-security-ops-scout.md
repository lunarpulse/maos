# Security/ops scout (HEAD 4657cace)
1. unsafe: 19 real blocks, all kernel-core sandbox (linux.rs:64, macos.rs:49, windows.rs:55-312). check-unsafe checks crate roots only (check_unsafe.rs:12).
2. deny.toml real + blocking; 35 multiple-versions skips; 2 unmaintained advisories ignored.
3. Secrets: plain env pass-through (providers/anthropic.rs:37,80). maos-secrets = LocalMasterKeyKms AES-GCM "dev/CI-only", NO keyring dep (Cargo.toml:8 promises keyring). main.rs:3079 FIXME(secrets).
4. Retired model IDs live: butler manifest:21 claude-3-haiku; researcher:47 claude-3-5-sonnet; templates spirit-rust/ts:12; examples:12; main.rs:3098 gpt-4o-mini, :3112 llama3.1:8b; provider-pricing.toml:18.
5. TLS/TOFU: only InMemoryTofuPinStore (tofu.rs:508); swap_serving_cert no prod caller (transport.rs:538-541); no OCSP/CRL (_ocsp_response ignored verifier.rs:231); `.dangerous()` verifier.
6. SANDBOX: T2 landlock+seccompiler code REAL (sandbox/linux.rs:64-120) but spawn_sandboxed( has ZERO production callers; Spirits launch via plain Command::new().spawn() (lifecycle/cli_wrapper/runtime.rs:461-472). admit_spirit builds SandboxSpec at 10+ sites never fed to spawner (security/mod.rs:39,485). T3 podman/docker only in smoke-t3-sandbox-5 (main.rs:6955-7027). T4 enum only. => FR5 headline promise NOT enforced at runtime.
7. Install: release.yml (tag v*) 3 targets, SHA256SUMS + Ed25519 sig via xtask RELEASE_SIGNING_KEY; never run (no v* tag). packaging/{homebrew,aur,deb,rpm} SCAFFOLD/PLACEHOLDER_SHA256, no workflow. Dockerfile distroless. `maosctl uninstall <spirit>` = forget_with_reason on memory rows only; no kernel uninstall removing files/MAOS_HOME (subcommands.rs:1434-1462; main.rs:8434-8620).
8. Air-gap: feature exists, default=["network"] (maos-bin Cargo.toml:15,29); check-air-gap in NO workflow; netns script exit 0 SKIP (:29-38). Release/Docker build network variant.
9. SECURITY.md: security@maos.dev, explicitly NO GPG key; CNA "submitted 2026-06-22 pending"; check-cna-registration checks file existence only (:51-76).
10. Pen-test: docs only, findings/ empty; Hold 1 open, not scheduled.
11. Error masking: ureq Err for HTTP>=400 -> IoError::Transport (io/mod.rs:78,105-108); ProviderRejected never constructed; map_err(|_| drops 61 in maos-bin, 14 kernel-core.
12. Fuzz: 3 targets; fuzz-ledger.json = {"records":[]} since 2026-06-22; weekly cron 600s x4 workers ≈ 0.67 CPU-hr/target/week vs 72 CPU-hr/90d floor; no 24h run ever.
