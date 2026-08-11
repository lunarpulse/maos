# Reproducing the four Loom substrate gates locally

**Story 13.6 (AC1) — the substrate close-out.** Until this document existed,
`grep -rn "maos_team_c" docs/ README.md` returned zero: the three-team ×
three-region substrate 13.6c shipped had no local runbook at all, and the two
in-tree headers that *did* describe a local run
(`crates/maos-loom-lite/tests/cross_region_live.rs`,
`crates/maos-loom-lite/tests/migration_live.rs`) still named a singular
`maos_test` database that CI stopped provisioning at 13.6c.

This is the runbook for all four substrate-bearing gates:

| Gate | Databases it needs | Variables the CI job exports |
|---|---|---|
| `check-cross-region-consensus` | `maos_shared`, `maos_team_{a,b,c}` | `MAOS_TEST_POSTGRES`, `..._A/_B/_C`, `..._TEAM_A/_TEAM_B/_TEAM_C` |
| `check-multi-region-slo` | `maos_team_{a,b,c}` | `..._A/_B/_C` |
| `check-multi-tenant-loom` | `maos_team_{a,b,c}` | `..._TEAM_A/_TEAM_B/_TEAM_C`, `MAOS_TEST_POSTGRES` (→ `maos_team_b`) |
| `check-reza-production-path` | `maos_team_{a,b}` | `..._TEAM_A/_TEAM_B` |

The contract table above is not prose — it is
`xtask/src/check_loom_substrate_drift.rs::CONTRACTS`, and
`check-loom-substrate-drift` reds if this table, the workflow, and the Rust
readers ever disagree.

## The topology rules the gate enforces

`check-loom-substrate-drift`'s `topology-value-distinctness` leg (Story 13.6,
AC1) reads the *values*, not just the keys:

1. **The region axis is pairwise distinct.** `MAOS_TEST_POSTGRES_{A,B,C}` must
   name three different databases.
2. **The team axis is pairwise distinct.** `MAOS_TEST_POSTGRES_TEAM_{A,B,C}`
   must name three different databases.
3. **Cross-axis aliasing is allowed only where it is ratified.**
   `..._A` and `..._TEAM_A` deliberately name one database — the signed
   `TeamEntry` carries one region and one datname, so "three teams × three
   regions" is **three databases, not nine**. `check-multi-tenant-loom`
   deliberately points the legacy singular `MAOS_TEST_POSTGRES` at
   `maos_team_b`. Both are in `RATIFIED_ALIASES`; anything else that collides
   is topology fraud.

This runbook deliberately provisions all names on **one PostgreSQL server**.
The structural oracle compares database names and the live witnesses use
`current_database()`; neither proves that same-named databases on different
servers are one physical database. Do not generalize the ratified aliases to a
multi-server layout without adding endpoint identity to the oracle.
4. **The shared stand-in is role-disjoint on the consensus gate.** There,
   `MAOS_TEST_POSTGRES` is `maos_shared` and may not be aliased onto a team
   database.

## Bring the substrate up

PostgreSQL ≥ 16 **with the pgvector extension available** works; CI uses `pgvector/pgvector:pg16`. A stock PostgreSQL installation without pgvector will fail during schema initialization. Two options.

### Docker (closest to CI)

```bash
docker run -d --name maos-loom -p 5432:5432 \
  -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=maos_team_a \
  pgvector/pgvector:pg16
export PGPASSWORD=postgres
until pg_isready -h 127.0.0.1 -U postgres -d maos_team_a; do sleep 1; done
for db in maos_team_b maos_team_c maos_shared; do
  createdb -h 127.0.0.1 -U postgres "$db"
done
```

### A local cluster (no container runtime)

```bash
export PATH=/usr/lib/postgresql/17/bin:$PATH   # or your distro's path
initdb -U postgres -D /tmp/maos_pg/data -A trust --locale=C --encoding=UTF8
postgres -D /tmp/maos_pg/data -p 5432 -k /tmp/maos_pg -c listen_addresses=127.0.0.1 &
for attempt in $(seq 1 30); do
  pg_isready -h 127.0.0.1 -p 5432 -U postgres && break
  [ "$attempt" -eq 30 ] && { echo "PostgreSQL did not become ready" >&2; exit 1; }
  sleep 1
done
for db in maos_team_a maos_team_b maos_team_c maos_shared; do
  createdb -h 127.0.0.1 -p 5432 -U postgres "$db"
done
```

The gates and live tests call `LoomLiteStore::init_schema` explicitly after
constructing the store; `LoomLiteStore::new` only creates the connection pool.
There is no migration command to run by hand.

## Export the substrate

```bash
PG=postgresql://postgres:postgres@127.0.0.1:5432
export MAOS_TEST_POSTGRES="$PG/maos_shared"
export MAOS_TEST_POSTGRES_A="$PG/maos_team_a"
export MAOS_TEST_POSTGRES_B="$PG/maos_team_b"
export MAOS_TEST_POSTGRES_C="$PG/maos_team_c"
export MAOS_TEST_POSTGRES_TEAM_A="$PG/maos_team_a"
export MAOS_TEST_POSTGRES_TEAM_B="$PG/maos_team_b"
export MAOS_TEST_POSTGRES_TEAM_C="$PG/maos_team_c"
```

⚠ **`check-multi-tenant-loom` is the exception**: its CI job exports
`MAOS_TEST_POSTGRES=$PG/maos_team_b`. Reproducing that gate exactly means
overriding the shared stand-in for that one invocation:

```bash
MAOS_TEST_POSTGRES="$PG/maos_team_b" cargo run -q -p xtask -- check-multi-tenant-loom
```

## Run each gate

Run from the workspace root. `check-loom-substrate-drift` reads
`.github/workflows/discipline.yml` relative to the current directory.

```bash
cd "$(git rev-parse --show-toplevel)"
cargo run -q -p xtask -- check-cross-region-consensus
cargo run -q -p xtask -- check-multi-region-slo
MAOS_TEST_POSTGRES="$PG/maos_team_b" cargo run -q -p xtask -- check-multi-tenant-loom
cargo run -q -p xtask -- check-reza-production-path
cargo run -q -p xtask -- check-loom-substrate-drift   # structural, needs no Postgres
```

`check-loom-substrate-drift` is hermetic on purpose — it judges the
declarations, so it reds on a substrate defect *before* anyone pays for a
Postgres.

## The operator lane (`PROVEN_LIVE_SIGNED`)

A live leg is `PROVEN_LIVE_SIGNED` only when the harness signed its own
transcript record with the **operator** audit key and the gate verified it.
CI holds no such key by ratified design (`evidence_ledger.rs` — *"a CI that
holds the operator key would be theatre"*), so `PROVEN_LIVE_SIGNED` is
reachable **only on an operator-run lane**:

```bash
maosctl audit keygen --output ~/.config/maos/audit-signing.key   # 0600, once
export MAOS_AUDIT_KEY=~/.config/maos/audit-signing.key
set -o pipefail
report=tests/reports/operator-lane-check-multi-tenant-loom.json
MAOS_TEST_POSTGRES="$PG/maos_team_b" cargo run -q -p xtask -- check-multi-tenant-loom --json \
  | tee "$report"
jq -e '
  .product_claim == "PROVEN"
  and any(.legs[];
    .name == "reza-three-team-three-region-journey"
    and .required == true
    and .evidence_state == "PROVEN_LIVE_SIGNED")
' "$report" >/dev/null
```

Without the key, ordinary attempted-green `AdvisorySubstrate` legs still run
but project `INDETERMINATE` because their signatures cannot be verified.
The required Reza journey is a narrower exception:
`check_multi_tenant_loom.rs::journey_successor` checks key availability before
launching and returns an `ABSENT` successor when no operator key is available.
The `detail` field distinguishes that deliberate no-key absence from a missing
test declaration; never key an evidence claim on `evidence_state` alone.

The `jq -e` check above is required even with `pipefail`: the development lane
may legitimately exit zero while `product_claim` is `NOT_PROVEN`.

## Tearing down

```bash
docker rm -f maos-loom                       # container
pg_ctl -D /tmp/maos_pg/data stop && rm -rf /tmp/maos_pg   # local cluster
```
