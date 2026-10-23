#!/usr/bin/env python3
"""Story 17-9 proven-red plants (FW-07 / §Proven-red controls).

Each plant is one exact text substitution against the candidate tree. This
script (a) renders every plant as a unified diff under `diffs/` (the CI
proven-red workflow applies one diff per matrix leg), and (b) optionally runs
each plant locally in a PRIVATE copy of the tree and records whether the named
test went red at the expected assertion.

    plants.py render --tree <repo>
    plants.py run --tree <private-copy> [--only ID ...]

The rows owned by the maos-bin slices (session verdict, recall/fetch typed
denial, MCP receipt, CLI refusal, confidentiality) are added to PLANTS below
once those slices land; every row names its command and expected assertion.
"""
import argparse
import difflib
import json
import os
import pathlib
import subprocess
import sys

CAP_QUOTA = "crates/maos-capability/src/cap_quota/mod.rs"
KERNEL_CAP = "crates/maos-kernel-core/src/capability/mod.rs"
WRITER = "crates/maos-kernel-core/src/capability/cap_audit/writer_task.rs"
WATCHDOG = "crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs"
FR4 = "crates/maos-audit/src/fr4_classifier.rs"
POSTURE = "crates/maos-audit/src/log_composition.rs"
LOADER = "crates/maos-audit/src/lib.rs"

AC1 = "cargo test --locked -p maos-kernel-core --test issuance_quota_window_17_9 -- --nocapture --test-threads=1"

CHARGE_BLOCK = """        // 2. Reserve quota only after policy allows the scope.
        let budget = self.quota.quota().budget_per_window;
        let charge = match self.quota.charge(spirit_pid) {
            Ok(charge) => charge,
            Err(exhausted) => {
                if exhausted.first_in_window {
                    self.audit_quota(spirit_pid, QuotaSignal::Exhausted, exhausted.window, budget);
                }
                return Err(CapError::IssuanceQuotaExhausted {
                    spirit_id: spirit_pid,
                });
            }
        };

"""

PLANTS = [
    {
        "id": "window-rollover",
        "control": "Window rollover",
        "edits": [(CAP_QUOTA, "        if window > slot.window {\n            *slot = Slot::new(window);\n        }\n", "")],
        "cmd": AC1 + " one_pid_keeps_its_capabilities_across_three_windows",
        "expect": "issuance 1 of 60000 for pid 7",
    },
    {
        "id": "inclusive-capacity",
        "control": "Inclusive capacity",
        "edits": [(CAP_QUOTA, "if slot.reserved == self.state.quota.budget_per_window {",
                   "if slot.reserved + 1 >= self.state.quota.budget_per_window {")],
        "cmd": AC1 + " one_pid_keeps_its_capabilities_across_three_windows && "
               "cargo test --locked -p maos-capability --test issuance_quota_17_9 capacity_is_inclusive",
        "expect": "issuance 60000 of 60000 for pid 7",
    },
    {
        "id": "charge-after-policy",
        "control": "Charge after policy",
        "edits": [
            (KERNEL_CAP, CHARGE_BLOCK, ""),
            (KERNEL_CAP, "        // 1. Policy check: derive Intent from the actual scope\n",
             CHARGE_BLOCK + "        // 1. Policy check: derive Intent from the actual scope\n"),
        ],
        "cmd": AC1 + " policy_denials_and_approval_required_never_charge",
        "expect": "right: Err(PolicyDenied)",
    },
    {
        "id": "refund-on-failed-issue",
        "control": "Refund on failed issue",
        "edits": [(KERNEL_CAP, "                self.quota.refund(charge);\n", "                drop(charge);\n")],
        "cmd": AC1 + " failed_or_late_signing_keeps_capacity_and_crossings_exact",
        "expect": "issuance 4 of 4 for pid 7",
    },
    {
        "id": "crossing-on-successful-issuance",
        "control": "Crossing on successful issuance",
        "edits": [
            (CAP_QUOTA, "pub struct Charge {\n    spirit_pid: u32,\n    window: u64,\n}",
             "pub struct Charge {\n    spirit_pid: u32,\n    window: u64,\n    owns: u8,\n}"),
            (CAP_QUOTA, "        slot.reserved += 1;\n        Ok(Charge { spirit_pid, window })",
             "        slot.reserved += 1;\n"
             "        // PLANT: the reservation that reaches a threshold owns its crossing.\n"
             "        let mut owns = 0;\n"
             "        if slot.reserved == self.state.pressure_at { owns |= PRESSURE_BIT; }\n"
             "        if slot.reserved == self.state.limit_at { owns |= LIMIT_BIT; }\n"
             "        Ok(Charge { spirit_pid, window, owns })"),
            (CAP_QUOTA, "                    if slot.issued >= threshold && slot.fired & bit == 0 {",
             "                    let _ = threshold;\n"
             "                    if charge.owns & bit != 0 && slot.fired & bit == 0 {"),
        ],
        "cmd": AC1 + " failed_or_late_signing_keeps_capacity_and_crossings_exact",
        "expect": "right: [(Pressure, 16)]",
    },
    {
        "id": "once-per-window",
        "control": "Once per window",
        "edits": [(CAP_QUOTA, "if slot.issued >= threshold && slot.fired & bit == 0 {",
                   "if slot.issued >= threshold {")],
        "cmd": AC1 + " pressure_limit_and_exhausted_fire_once_per_window",
        "expect": "right: 1",
    },
    {
        "id": "rearm-on-rollover",
        "control": "Re-arm on rollover",
        "edits": [(CAP_QUOTA, "        if window > slot.window {\n            *slot = Slot::new(window);\n        }\n",
                   "        if window > slot.window {\n            let fired = slot.fired;\n"
                   "            *slot = Slot::new(window);\n            slot.fired = fired;\n        }\n")],
        "cmd": AC1 + " pressure_limit_and_exhausted_fire_once_per_window",
        "expect": "right: [(Pressure, 48000)]",
    },
    {
        "id": "epoch-never-decreases",
        "control": "Epoch never decreases",
        "edits": [(CAP_QUOTA,
                   "        let window = self\n            .window()\n            .max(slot.window)\n"
                   "            .max(self.state.swept.load(Ordering::Acquire));\n        if window > slot.window {",
                   "        let window = self.window();\n        if window != slot.window {")],
        "cmd": AC1 + " racing_issuers_never_exceed_or_restore_capacity",
        "expect": "right: Err(IssuanceQuotaExhausted { spirit_id: 7 })",
    },
    {
        "id": "atomicity",
        "control": "Atomicity",
        "edits": [
            (CAP_QUOTA, "    fn reserve(&self, spirit_pid: u32) -> Result<Charge, Exhausted> {\n",
             "    fn reserve(&self, spirit_pid: u32) -> Result<Charge, Exhausted> {\n"
             "        // PLANT: capacity read outside the entry guard; the racing pair\n"
             "        // rendezvous between the read and the write.\n"
             "        let free = self.state.slots.get(&spirit_pid).map_or(true, |s| {\n"
             "            s.reserved < self.state.quota.budget_per_window\n"
             "        });\n"
             "        if matches!(std::thread::current().name(), Some(\"17-9-holder\" | \"17-9-contender\")) {\n"
             "            static PLANT_RENDEZVOUS: std::sync::Barrier = std::sync::Barrier::new(2);\n"
             "            PLANT_RENDEZVOUS.wait();\n"
             "        }\n"),
            (CAP_QUOTA, "if slot.reserved == self.state.quota.budget_per_window {", "if !free {"),
        ],
        "cmd": AC1 + " racing_issuers_never_exceed_or_restore_capacity",
        "expect": "right: 1",
    },
    {
        "id": "strictly-older-sweep",
        "control": "Strictly-older sweep",
        "edits": [(CAP_QUOTA, "retain(|_, slot| slot.window >= window)", "retain(|_, slot| slot.window == window)")],
        "cmd": "cargo test --locked -p maos-capability --lib cap_quota",
        "expect": "overlapping_sweeps_never_remove_a_slot_of_the_current_or_a_later_window",
    },
    {
        "id": "quota-rows-reach-tl",
        "control": "Quota rows reach the TL",
        "edits": [(WRITER,
                   "            log.insert_frame_event(\n                FrameKind::CapabilityInvocation,\n"
                   "                *spirit_pid,\n                None,\n                signal.intent(),\n"
                   "                payload.as_bytes(),\n                FrameOrigin::Kernel,\n            );\n",
                   "            let _ = (spirit_pid, signal, payload);\n")],
        "cmd": AC1 + " quota_rows_reach_the_transparency_log",
        "expect": "right: 6",
    },
    {
        "id": "drop-accounting",
        "control": "Drop accounting",
        "edits": [(KERNEL_CAP, "record_send_error(cap_audit::AuditDropSite::Quota, &error)",
                   "record_send_error(cap_audit::AuditDropSite::Issue, &error)")],
        "cmd": "cargo test --locked -p maos-kernel-core --test audit_sink_truth_16_5",
        "expect": "audit_sink_truth_lifecycle_healthy_then_closed",
    },
    {
        "id": "fr4-disposition",
        "control": "FR4 disposition",
        "edits": [(FR4,
                   "    WriterShapeEntry {\n"
                   "        // Story 17-9 — the cap-audit writer's tokenless issuance-quota signals\n"
                   "        // (`cap.quota:pressure` / `:limit` / `:exhausted`), operator-facing.\n"
                   "        site: (\"writer_task.rs\", \"write_to_transparency_log\", 5),\n"
                   "        kind: 7,\n        token: TokenColumn::Absent,\n"
                   "        intent: WriterIntent::Prefix(\"cap.quota:\"),\n"
                   "        shape: Some(&[\n            (\"window\", PayloadType::Num),\n"
                   "            (\"used\", PayloadType::Num),\n            (\"budget\", PayloadType::Num),\n"
                   "        ]),\n    },\n", "")],
        "cmd": "cargo test --locked -p maos-audit --test fr4_classifier_16_2",
        "expect": "every_writer_site_demanding_a_disposition_has_one",
    },
    {
        "id": "posture-delta",
        "control": "Posture delta",
        "edits": [(POSTURE, "if frame_kind == \"capability.invocation\" && intent.starts_with(\"cap.issue.\") {",
                   "if frame_kind == \"capability.invocation\" {")],
        "cmd": "cargo test --locked -p maos-audit --lib log_composition",
        "expect": "right: 2",
    },
    {
        "id": "mint-after-bucket",
        "control": "Mint after bucket (FW-10)",
        "edits": "reverse-W",
        "cmd": "cargo test --locked -p maos-kernel-core --features fixture_replay --test schedule_watchdog_fr26 schedule_17_9",
        "expect": "minted only for admitted fires",
    },
    {
        "id": "ratification-loader-payload",
        "control": "Ratification loader (select payload)",
        "edits": [(LOADER, "\"SELECT rowid, intent, payload_redacted FROM transparency_log \\",
                   "\"SELECT rowid, intent, payload FROM transparency_log \\")],
        "cmd": "cargo test --locked -p maos-audit --test ratification_loader_17_9",
        "expect": "every kind-28 row is classified",
    },
    {
        "id": "ratification-loader-decode-all",
        "control": "Ratification loader (decode every kind-28 row)",
        "edits": [(LOADER, "        if is_kind28_receipt_intent(&intent) {\n            continue;\n        }\n", "")],
        "cmd": "cargo test --locked -p maos-audit --test ratification_loader_17_9",
        "expect": "every kind-28 row is classified",
    },
]

SESSION = "crates/maos-bin/src/spirit_session.rs"
RECALL = "crates/maos-bin/src/recall_mediation.rs"
SPAWN = "crates/maos-bin/src/worker_spawn.rs"
TL = "crates/maos-iac/src/adapter/transparency_log.rs"
UNDELEGATED = "MAOS_17_2_CGROUP_MODE=undelegated "
WASM = "cargo test --locked -p maos-bin --features network,wasm-host "

# The maos-bin rows. `kind` selects the CI substrate: `wasm` legs run in the
# undelegated cgroup unit with the guest fixtures fetched; `t3` runs on the
# provisioned T3 substrate. `local_cmd` is this host's equivalent.
PLANTS += [
    {
        "id": "session-verdict",
        "control": "Session verdict",
        "kind": "wasm",
        "edits": [(SESSION, "GovernedMintError::QuotaExhausted => Some(\"exhausted\"),",
                   "GovernedMintError::QuotaExhausted => None,")],
        "cmd": WASM + "--test spawned_scope_17_3c -- --nocapture --test-threads=1 "
               "a_delivery_needs_its_iac_send_grant wasm_delivery_is_refused_when_its_issuance_window_is_exhausted",
        "expect": ["right: Some(\"exhausted\")", "left: Null"],
    },
    {
        "id": "recall-typed-denial",
        "control": "Recall/fetch typed denial",
        "kind": "wasm",
        "edits": [
            (RECALL,
             "        self.governed(pid, Scope::LogRecall, |token| {\n"
             "            self.adapter.recall_attributed(pid, filter, token)\n        })\n",
             "        match self.governed(pid, Scope::LogRecall, |token| {\n"
             "            self.adapter.recall_attributed(pid, filter, token)\n        }) {\n"
             "            // PLANT: a refused mint becomes an empty page.\n"
             "            Err(LogRecallError::CapabilityDenied) => Ok(LogRecallPage {\n"
             "                entries: Vec::new(),\n                next_cursor: None,\n"
             "                scope: maos_domain::log_recall::RecallScope::Participant,\n            }),\n"
             "            other => other,\n        }\n"),
            (RECALL,
             "            self.adapter.fetch_attributed(pid, id, token)\n        });\n",
             "            self.adapter.fetch_attributed(pid, id, token)\n        });\n"
             "        // PLANT: a refused mint becomes an empty fetch.\n"
             "        let result = match result {\n"
             "            Err(LogRecallError::CapabilityDenied) => Ok(LogFetchResponse::new(\n"
             "                id, 0, maos_domain::log_recall::FrameKindLabel::TaskAssign,\n"
             "                String::new(), Vec::new(), None, FrameOrigin::Kernel,\n            )),\n"
             "            other => other,\n        };\n"),
        ],
        "cmd": "cargo test --locked -p maos-bin --features network --test recall_mediation_17_3d; native=$?; "
               + UNDELEGATED + "MAOS_17_3D_QUOTA_CHILD=recall " + WASM
               + "--test recall_17_3d exhausted_quota_has_receipt_without_tokenless_fallback_call "
               "-- --exact --nocapture --test-threads=1; wasm=$?; exit $(( native | wasm ))",
        "expect": ["right: Err(CapabilityDenied)", "guest-visible denial contains no pid, budget or window"],
    },
    {
        "id": "mcp-receipt",
        "control": "MCP receipt",
        "kind": "plain",
        "edits": [(SPAWN, "    if result.is_err() {\n        let payload = serde_json::to_vec(&McpRefusalPayload",
                   "    if false && result.is_err() {\n        let payload = serde_json::to_vec(&McpRefusalPayload")],
        "cmd": "cargo test --locked -p maos-bin --features network --test mcp_mint_receipt_17_9; a=$?; "
               "cargo test --locked -p maos-bin --features network --bin maos live_mcp_; b=$?; exit $(( a | b ))",
        "expect": ["exactly one MCP refusal receipt", "exactly one receipt for the one refused mint"],
    },
    {
        "id": "cli-refusal",
        "control": "CLI refusal",
        "kind": "t3",
        "edits": [(SPAWN,
                   "                GovernedMintError::QuotaExhausted => {\n"
                   "                    return Err(format!(\n"
                   "                        \"maos run: cli_wrapper spawn REFUSED: {e}; a quota refusal never \\\n"
                   "                         proceeds without a kernel token (Story 17-9 FW-06)\"\n"
                   "                    )\n                    .into());\n                }\n",
                   "                // PLANT: exhaustion treated like `Mediation` (D1 proceed).\n"
                   "                GovernedMintError::QuotaExhausted => {}\n")],
        "cmd": "cargo test --locked -p maos-bin --features network --test cli_wrapper_exhaustion_17_9 "
               "exhaustion_refuses_before_workload -- --nocapture --test-threads=1",
        "local_cmd": "cargo build --locked -p worker --bin worker-cli-fixture && "
                     "source /mnt/build/17-9-cli-signing-only-d/t3.env && env -u CI "
                     "cargo test --locked -p maos-bin --features network --test cli_wrapper_exhaustion_17_9 "
                     "exhaustion_refuses_before_workload -- --nocapture --test-threads=1",
        "expect": "quota exhaustion must refuse before workload launch",
    },
    {
        "id": "confidentiality",
        "control": "Confidentiality",
        "kind": "wasm",
        "edits": [(TL, "pub const SPIRIT_DISCLOSABLE_RAW_KINDS: [i64; 8] = [0, 1, 2, 3, 4, 5, 6, 18];",
                   "pub const SPIRIT_DISCLOSABLE_RAW_KINDS: [i64; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 18];")],
        "cmd": "cargo test --locked -p maos-bin --features network --test recall_mediation_17_3d "
               "quota_audit_and_mcp_receipts_are_operator_visible_but_never_spirit_disclosed -- --exact; a=$?; "
               + UNDELEGATED + WASM + "--test recall_17_3d "
               "kernel_quota_rows_and_mcp_refusals_never_cross_the_guest_recall_boundary "
               "-- --exact --nocapture --test-threads=1; b=$?; exit $(( a | b ))",
        "expect": ["thread 'quota_audit_and_mcp_receipts_are_operator_visible_but_never_spirit_disclosed'",
                   "neither operator-only metadata nor either payload reaches the guest"],
    },
]


def expects(plant):
    return plant["expect"] if isinstance(plant["expect"], list) else [plant["expect"]]


def planted(tree: pathlib.Path, plant):
    """Return {path: (original, planted)} for one plant."""
    if plant["edits"] == "reverse-W":
        # The base watchdog (mint before the rate bucket) with this story's
        # line-for-line comment rewrite kept: reverse only W's move.
        orig = (tree / WATCHDOG).read_text()
        bucket = (
            "            // Step 3 — consume admission before minting or serializing the row.\n"
            "            // Serialization of this fixed record is not expected to fail.\n"
            "            let bucket_key = (scb.spirit_id.clone(), entry.id.clone());\n"
            "            let bucket = self\n                .rate_limits\n                .entry(bucket_key)\n"
            "                .or_insert_with(|| Arc::new(ScheduleBucket::new(entry.rate_limit_per_hour)))\n"
            "                .value()\n                .clone();\n"
            "            if !bucket.try_consume() {\n                continue;\n            }\n\n")
        assert orig.count(bucket) == 1, "watchdog bucket block not found"
        new = orig.replace(bucket, "")
        anchor = "            let intent_str = format!(\"schedule.fire:{}\", entry.id);\n"
        assert new.count(anchor) == 1
        new = new.replace(anchor, bucket.replace("// Step 3 — consume admission before minting or serializing the row.\n            // Serialization of this fixed record is not expected to fail.\n", "// PLANT: rate-limit consume AFTER minting (pre-FW-10 order).\n") + anchor)
        return {WATCHDOG: (orig, new)}
    files = {}
    for path, old, new in plant["edits"]:
        before = files.get(path, ((tree / path).read_text(),) * 2)
        current = before[1]
        assert current.count(old) == 1, f"{plant['id']}: {path}: expected one match of {old!r}"
        files[path] = (before[0], current.replace(old, new))
    return files


def render(tree: pathlib.Path, out: pathlib.Path):
    out.mkdir(parents=True, exist_ok=True)
    index = []
    for plant in PLANTS:
        chunks = []
        for path, (orig, new) in planted(tree, plant).items():
            chunks.extend(difflib.unified_diff(
                orig.splitlines(keepends=True), new.splitlines(keepends=True),
                fromfile=f"a/{path}", tofile=f"b/{path}"))
        (out / f"{plant['id']}.diff").write_text("".join(chunks))
        index.append({
            "id": plant["id"], "control": plant["control"], "kind": plant.get("kind", "plain"),
            "cmd": plant["cmd"], "expect": expects(plant),
        })
    (out / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    print(f"rendered {len(index)} plants into {out}")


def run(tree: pathlib.Path, only, log_dir: pathlib.Path):
    log_dir.mkdir(parents=True, exist_ok=True)
    results = []
    for plant in PLANTS:
        if only and plant["id"] not in only:
            continue
        files = planted(tree, plant)
        for path, (_, new) in files.items():
            (tree / path).write_text(new)
        try:
            proc = subprocess.run(["bash", "-o", "pipefail", "-c", plant.get("local_cmd", plant["cmd"])],
                                  cwd=tree, capture_output=True, text=True, env=os.environ.copy())
        finally:
            for path, (orig, _) in files.items():
                (tree / path).write_text(orig)
        output = proc.stdout + proc.stderr
        (log_dir / f"{plant['id']}.log").write_text(output)
        red = proc.returncode != 0
        named = all(text in output for text in expects(plant))
        verdict = "RED-AT-NAMED" if red and named else ("RED-OTHER" if red else "GREEN")
        results.append({"id": plant["id"], "rc": proc.returncode, "verdict": verdict})
        print(f"{plant['id']}: {verdict} (rc={proc.returncode})", flush=True)
    (log_dir / "results.json").write_text(json.dumps(results, indent=2) + "\n")
    return 0 if all(r["verdict"] == "RED-AT-NAMED" for r in results) else 1

DIFFS = "_bmad-output/implementation-artifacts/17-9-evidence/plants/diffs"
FIXTURES = ("crates/maos-wasm-host/guests/echo-spirit crates/maos-wasm-host/guests/routed-relay "
            "crates/maos-wasm-host/guests/recall-probe crates/maos-wasm-host/guests/compat-echo-2-0 "
            "crates/maos-wasm-host/guests/equiv-fixture/native-twin")

VERIFY = r'''set +e
sha=$(sha256sum "$DIFFS/$ID.diff" | cut -d' ' -f1)
echo "::notice title=17-9 provenance $ID (${{ matrix.image }})::sha=${GITHUB_SHA} tree=$(git rev-parse HEAD^{tree}) plant_sha256=${sha} rustc=$(rustc --version | tr ' ' '_')"
git apply --whitespace=nowarn "$DIFFS/$ID.diff" || { echo "::error title=PLANT-DID-NOT-APPLY $ID::git apply failed"; exit 1; }
case "$KIND" in
  wasm) python3 scripts/run_cgroup_ceiling_17_2.py run --mode undelegated --unit "maos-17-2-red-17-9-$ID" --timeout 2400 -- \
          bash -o pipefail -c "$CMD" > red.log 2>&1 ;;
  t3)   python3 scripts/run_cgroup_ceiling_17_2.py run --mode delegated --unit "maos-17-7-red-17-9-$ID" --timeout 1800 --rootless-canary -- \
          bash -o pipefail -c "$CMD" > red.log 2>&1 ;;
  *)    bash -o pipefail -c "$CMD" > red.log 2>&1 ;;
esac
rc=$?
missing=""
while IFS= read -r want; do
  [ -z "$want" ] && continue
  grep -qF -- "$want" red.log || missing="${missing}[${want}] "
done <<< "$EXPECT"
grep -E 'panicked at|^test .* FAILED$' red.log | head -6 | while IFS= read -r line; do
  echo "::notice title=17-9 red evidence $ID (${{ matrix.image }})::${line}"
done
if [ "$rc" -ne 0 ] && [ -z "$missing" ]; then
  first=$(printf '%s\n' "$EXPECT" | head -n1)
  echo "::error title=PROVEN-RED $ID (${{ matrix.image }})::rc=${rc} red at the named assertion: $(grep -m1 -F -- "$first" red.log | cut -c1-300)"
elif [ "$rc" -eq 0 ]; then
  echo "::error title=NOT-RED $ID (${{ matrix.image }})::the plant was not detected (rc=0)"
else
  echo "::error title=RED-OTHER $ID (${{ matrix.image }})::rc=${rc} but missing ${missing}"
fi
exit 1
'''


def workflow(branch: str) -> str:
    """The branch-only proven-red workflow: one matrix leg per control."""
    legs = []
    for plant in PLANTS:
        legs.append({"id": plant["id"], "control": plant["control"], "kind": plant.get("kind", "plain"),
                     "cmd": plant["cmd"], "expect": "\n".join(expects(plant))})
    q = json.dumps
    lines = [
        "# Story 17-9 — branch-only proven-red workflow (D-17-9-FW-07 A, under",
        "# D-17-3d-FW-10's terms). Lives ONLY on `ci/17-9-verify` commit c2 and never",
        "# lands on main. One matrix leg per §Proven-red control: the leg applies its",
        "# plant diff, runs the named suite and must be RED at the named assertion",
        "# (annotation PROVEN-RED); NOT-RED / RED-OTHER annotations are failures of the",
        "# proof. Read-only permissions, no secrets. Generated by",
        "# `_bmad-output/implementation-artifacts/17-9-evidence/plants/plants.py workflow`.",
        "name: 17-9-proven-red",
        "on:",
        "  push:",
        f"    branches: [{q(branch)}]",
        "permissions:",
        "  contents: read",
        "jobs:",
        "  proven-red:",
        "    strategy:",
        "      fail-fast: false",
        "      matrix:",
        "        image: [ubuntu-24.04]",
        "        leg:",
    ]
    for leg in legs:
        lines.append("          - " + q(leg))
    lines += [
        "    name: red ${{ matrix.leg.id }}",
        "    runs-on: ${{ matrix.image }}",
        "    timeout-minutes: 90",
        "    env:",
        "      ID: ${{ matrix.leg.id }}",
        "      KIND: ${{ matrix.leg.kind }}",
        "      CMD: ${{ matrix.leg.cmd }}",
        "      EXPECT: ${{ matrix.leg.expect }}",
        f"      DIFFS: {DIFFS}",
        "    steps:",
        "      - uses: actions/checkout@v5",
        "        with:",
        "          persist-credentials: false",
        "      - uses: dtolnay/rust-toolchain@v1",
        "        with:",
        "          toolchain: stable",
        "          targets: wasm32-wasip2",
        "      - uses: Swatinem/rust-cache@v2",
        "        with:",
        "          key: 17-9-red-${{ matrix.leg.kind }}-${{ hashFiles('**/Cargo.lock') }}",
        "      - name: Keep build and image temporaries off /tmp",
        "        run: echo \"TMPDIR=$RUNNER_TEMP\" >> \"$GITHUB_ENV\"",
        "      - if: ${{ matrix.leg.kind == 'wasm' }}",
        "        name: WASM prerequisites (pinned assembler, offline guest fixtures)",
        "        run: |",
        "          cargo install --locked wasm-tools --version 1.259.0",
        "          cargo fetch --locked",
        f"          for fixture in {FIXTURES}; do",
        "            cargo fetch --locked --manifest-path \"$fixture/Cargo.toml\"",
        "          done",
        "      - if: ${{ matrix.leg.kind == 't3' }}",
        "        uses: ./.github/actions/provision-t3-test-substrate",
        "      - if: ${{ matrix.leg.kind == 't3' }}",
        "        name: T3 prerequisites (Worker CLI fixture, locked graph)",
        "        run: |",
        "          cargo build --locked -p worker --bin worker-cli-fixture",
        "          cargo fetch --locked",
        "      - name: Apply the plant and require RED at the named assertion",
        "        shell: bash",
        "        run: |",
    ]
    lines += ["          " + line if line else "" for line in VERIFY.splitlines()]
    lines += [
        "      - uses: actions/upload-artifact@v4",
        "        if: always()",
        "        with:",
        "          name: red-${{ matrix.leg.id }}",
        "          path: red.log",
        "          if-no-files-found: warn",
    ]
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=["render", "run", "workflow"])
    parser.add_argument("--tree", type=pathlib.Path, default=pathlib.Path("."))
    parser.add_argument("--out", type=pathlib.Path, default=pathlib.Path(__file__).parent / "diffs")
    parser.add_argument("--logs", type=pathlib.Path, default=pathlib.Path(__file__).parent / "local")
    parser.add_argument("--only", nargs="*")
    parser.add_argument("--branch", default="ci/17-9-verify")
    args = parser.parse_args()
    if args.mode == "render":
        render(args.tree, args.out)
        return 0
    if args.mode == "workflow":
        sys.stdout.write(workflow(args.branch))
        return 0
    return run(args.tree, args.only, args.logs)


if __name__ == "__main__":
    sys.exit(main())
