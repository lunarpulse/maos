# 11. Deployment Topologies

Two deployment topologies. Configuration alone composes the substrate into either.

## 11.1 Single-user (single Host)

The J0 / J-Butler / J-Researcher / J1 baseline. One MAOS Host on a laptop or workstation. Up to 5–10 Spirits scheduled cooperatively, with subprocess-form Spirits each in their own cgroups v2 process. Loom-lite optional (typically used by founder-loop distillation-shipping Spirits, not by Butler / Researcher).

**Configuration shape:**
- Single Host runs the kernel with all 5 reference Spirit classes available.
- Default Spirit set: Butler + Researcher + Architect + Observer (loaded on demand).
- Founder loop deployment: add Orchestrator + Developer-Worker + Reviewer-Worker skill packs loaded into agent CLI processes via CliWrapperSpirit.
- Persistence: SQLite for Transparency Log, Approval Decision Log, Journal. Optional Postgres for Loom-lite if the user has founder-loop Spirits opting in.
- Provider: any subset of (Anthropic / OpenAI / local-LLM via Ollama / etc.) per `maos-providers` config.
- Networking: localhost-only by default; A2A loopback profile for the founder-loop multi-CLI pattern.

## 11.2 Diagnostic-architect pair (bilateral 2-Host)

The J4 Mira-Nash deployment. Exactly two MAOS Hosts: Mira on a prod-edge node (read-only on production runtime; RW only on runtime knobs with approval; bash-exec whitelist for containment), Nash in a dev-environment (RW source repo, prompts on every deploy). Hosts are pre-paired at deployment time with each other's mTLS certificate fingerprint.

**Configuration shape:**
- Host A (prod-edge): kernel runs Mira + Observer (sentinel-style narrow capability set focused on production telemetry).
- Host B (dev-environment): kernel runs Nash + Observer + optionally Orchestrator/Workers for the team's coding workflow.
- Loom-lite: a single Postgres+pgvector instance accessible from both Hosts as MCP-Streamable-HTTP. Houses the ADR-pattern library, fix templates, regression-test references. Curation is Spirit-side; Nash decides what is worth persisting.
- Bilateral A2A: configured at deployment with the peer's mTLS cert fingerprint. Per-frame ADR-012 typed-intent consent: Mira's send-allowlist includes `diagnosis-handoff:read-only-evidence`, `cross-environment-telemetry-query`; Nash's accept-allowlist mirrors. Code-mutation-directive frames are blocked at the kernel boundary.
- Mobile push: HTTP push to Elena's phone for high-confidence diagnoses; the operator can approve from the mobile surface.

## 11.3 What's the same across both topologies

The substrate's invariants. The Spirit ABI. The manifest schema. The Capability Registry's mediation policy. The Transparency Log shape. The distillation pattern. The Approval Manager's UX. The hot-swap mechanism. The 14 invariants. The 39 ADRs (with their phased Status tags per §12.0). **Topology is configuration; architecture is invariant.** This is the substrate-positioning claim cashed: same primitives compose into either deployment.

## 11.6 Standing Host (server)

A Host that stays up is started from one file: `maos serve --config <host.toml>` (ADR-071). The file holds the
Host's settings, each with one key and one registered `MAOS_*` name, and the standing cast as
`[[host.spirits]]`. The cast is fed through the existing topology and installed boot paths, so a restart (a SIGTERM
drain or a SIGKILL) brings back exactly the Spirits the file names; there is no separate restore mechanism.
A bad file, an older config file that disagrees with it, or a region the signed cohort manifest contradicts is
a typed boot refusal, never a warning. The data roots resolve by one precedence (an explicit `MAOS_HOME`, then XDG,
then the legacy home).

**As a service (Story 22-2).** The deb and the rpm ship `maos.service`: it runs `maos init --plain` (the one minter of the operator door config) and then `maos serve --config /etc/maos/host.toml`, restarts on failure except exit 78, and stops within 30 s. `maosctl status` is the probe: one authenticated read of `GET /v1/daemon`.

**In a container (Story 22-3).** The image (`ghcr.io/<owner>/maos:<tag>`, multi-arch, cosign-signed) holds the signed `maos`, `maosctl` and `maos-wasm-runner`, the in-tree Spirit manifests and a default Host file naming nash and observer, so a pulled image serves a real cast with no provider key. Two deployments ship: `packaging/compose/compose.yaml` (a one-shot `init`, then `maos serve`, on the named volume `maos-home`) and the Helm chart `packaging/helm/maos` (a one-replica StatefulSet with a PVC at `/var/lib/maos`, the Host file from a ConfigMap, secrets from values or an existing Secret, an init container, read-only root filesystem, uid/gid/fsGroup 65532). **The door never leaves loopback:** neither deployment publishes a port or renders a Service, so the probe is an exec of `maosctl status` (readiness, liveness, compose healthcheck), not an HTTP probe. **T3 Workers are not supported in a container or pod:** they need a rootless container runtime, and neither mounts a container socket or requests privilege to provide one. Image size and idle memory are bounded per PR (NFR-Ops-14).
