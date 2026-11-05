# AG-1: Air-gap deployment

## Deployment model (20-2, D-20-2-01 A)

Deploy the **shipped default build** in a host-enforced network namespace with
no external interfaces. Transfer the signed release, Spirit bundles and replay
cassettes from a connected staging host. Verify them before installation; see
[install MAOS](../maos.dev/install.md). Network isolation, not a feature flag,
is the security boundary. A live remote inference provider cannot work offline.

`--no-default-features --features air-gap` is a **compile-shape control only**.
It is not a release artifact or a bootable offline runtime. Its `run` refuses
with `BuildShapeUnsupported` (exit 2), without writing a fake Transparency-Log
entry. Older binaries' `transparency.airgap-stub.log` is still removed by purge.
The closure includes `ureq`, `tokio-postgres`, rustls and Tokio networking.
At cbed4dad, the symbol scan also found `TcpListener` and `getaddrinfo`; therefore
`check-air-gap` is advisory under the operator's conditional D-20-2-02 C.
There is no claim of “zero networking code”.

## Behavioral witness

The blocking discipline job `air-gap-netns-witness` consumes the dev-signed
`release-dry-run` default binary. It calls:

```bash
bash tests/air-gap-netns-corroborate.sh "$PWD/dist/maos-linux-amd64" /tmp/maos-netns-proof
```

Prerequisites: Linux, `sudo -n`, util-linux (`unshare`, `setpriv`), `strace`,
Python 3, the default executable and the committed J0 shell-intro cassette.
Run as an ordinary user, **not by putting sudo around the entire script**.
Only namespace creation is privileged; `setpriv` drops uid/gid/groups before
strace or the workload runs. HOME and XDG roots are isolated.

The witness performs `maos init`, an actual `maos shell` replay session, checks
the expected hello-spirit response and increases in SQLite Transparency-Log
row count, and follows all children with `strace -f -yy -e trace=network`.
`connect`, `sendto` and `sendmsg` involving AF_INET/AF_INET6 (including annotated
connected TCP/UDP descriptors) must be zero. AF_UNIX and other families are
printed separately, not mistaken for outbound networking. A TCP canary in the
same namespace must be observed. An empty trace is a failure.

Declared CI fails on every missing prerequisite. A local missing prerequisite
returns 77 and prints `NOT RUN`; it is **not passing evidence**. Any real outbound
attempt by the final workload is a product defect: STOP and record a decision
with its trace, never add an allowlist. `--plant-outbound` as the third argument
injects a real TCP attempt into the traced session process tree and must fail.

The operator-only branch recipe is
`_bmad-output/implementation-artifacts/20-2-deb-airgap-docker-and-formula-corrections-evidence/ci/ci-20-2-verify.yml`.
It belongs under `.github/workflows/` only on `ci/20-2-verify`, never on main.
The hosted netns result and plant remain **unverified** until its run artifacts
are recorded. Wiring a job is not evidence that the product passes it.

## Spirit-author capability guidance

The default build does not stop issuing networking scopes merely because the
host is isolated. Use the actual scopes in
`crates/maos-domain/src/invariants/i1.rs`: `NetHttps { domain }`,
`ProviderInfer { provider }`, `FsRead { subtree }`, `FsWrite { subtree }`,
`MemRead { scope }`, `MemWrite { scope }` and `McpCall { server, tool }`.
There are no `NetworkOutbound` or `RegistryPoll` variants. Declare only needed
capabilities. Omit `net.https` and remote MCP transports for offline Spirits;
`provider.complete`/`ProviderInfer` can still be issued to the replay provider
(the shell host issues its selected provider token). A token is authorization,
not a transport or a promise that the network exists. Never put provider secrets
into replay fixtures. Import signed Spirit bundles offline using the documented
`maosctl install --source <bundle>` flow; do not attempt registry polling.

## Journaling and limitations

The default build records real session, lifecycle and governance entries in
`$MAOS_HOME/audit/transparency.sqlite`. Preserve the log for external review;
`maos purge --keep-log --yes` is the explicit retention option. The compile-shape
build's `run` does not boot and does not provide TL parity.

Namespace isolation blocks external communication, not filesystem/IPC abuse.
Retain capability checks and sandbox policy. Live registry sync, remote LLMs,
HTTP MCP, cross-host A2A and mobile push are unavailable offline; local replay
and local-only activities are the witnessed deployment model, not replacements
for those online services.
