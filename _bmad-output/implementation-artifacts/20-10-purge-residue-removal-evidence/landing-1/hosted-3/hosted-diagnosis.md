## Verdict

**The repaired witness now reaches production removal. Base is red for both intended reasons; the planted no-op is also red for its intended reason. Both heads—and the byte-restored plant—then fail on real CID-custody reclamation, not the previous ancestor-lock defect.**

Line numbers below refer to each job’s raw `gh api …/jobs/<id>/logs` output.

| Leg | First real failing assertion/step | Classification | Intended result? |
|---|---|---|---|
| [24.04 head](https://github.com/lunarpulse/maos/actions/runs/41803510235/job/126486336683) | **L6246–6247**, `root_crash.rs:280`: `CID custody survived: /run/user/1001/maos-worker-state/process-17374-9d4f1dcd/ffae2a0c` | **PRODUCTION defect, AC5 custody cleanup.** Source-established ordering bug described below | **No:** head must pass |
| [26.04 head](https://github.com/lunarpulse/maos/actions/runs/41803510235/job/126486336759) | **L6246–6247**, same assertion, `process-17415-d861c3ac/c28987df` | **Same PRODUCTION defect**, not an Ubuntu-specific failure | **No** |
| [24.04 base](https://github.com/lunarpulse/maos/actions/runs/41803510235/job/126486336797) | **L5479–5480**, `root_crash.rs:358`: `dead-creator empty leaf survived`; subsequently **L5496–5499**, `:274`: `owned CID … survived purge`, actual `Some(0)`, expected `Some(1)` | **Expected pre-20-10 production absence**, not a harness/witness defect | **Yes, both required diagnostics.** The job correctly concludes success because the negative control is accepted |
| [24.04 plant](https://github.com/lunarpulse/maos/actions/runs/41803510235/job/126486336742) | Initial mutation: **L5489–5492**, `root_crash.rs:274`: `owned CID … survived purge`, actual `Some(0)`. **Actual job-failing phase:** restored run **L6759–6760**, `:280`: `CID custody survived` | Initial red is the **intended planted removal defect**. Restored failure is the **same AC5 PRODUCTION defect** as both heads | **Plant red: yes. Restore green: no** |

## Per-leg evidence

### Both heads

- **L4776:** hermetic contracts pass **17/17** on each image.
- **L4821–4823:** real delegated service, `Delegate=yes`, `cpu,memory,pids`, and successful provisioned rootless-Podman canary.
- **L6238–6242:** real `purge_cgroup_native_binding_and_dead_creator_twins` passes on each image.
- **L6246–6247:** first actual failure is **spawn custody surviving**.
- **L6545–6546:** each head finishes **34 run, 33 passed, 1 failed**.

The execution reaches `root_crash.rs:280` only after:
1. dry purge succeeds (`:247–251`);
2. confirmed purge satisfies the exit/summary convention (`:265–272`);
3. **both owned CIDs are absent** (`:273–275`);
4. the foreign live-home CID and label-less impostor remain (`:276–278`).

Thus the failure is later than container attribution/removal and cross-home isolation. It is not the prior `StoreInUse` failure.

### Base

- **L5479–5480:** intended AC4 diagnostic.
- **L5496–5499:** intended AC1 diagnostic.
- **L5512:** two tests run, both fail.
- No subsequent error occurs in the delegated-witness step; GitHub reports the base job **success**.

This satisfies **both** workflow requirements at `ci-20-10-verify.yml:174–175`. The base tests are intentionally red; the control job is intentionally green.

### Plant

- **L234:** no-op patch SHA validation succeeds.
- **L5489–5492:** the initial run reaches the exact required `owned CID .* survived purge` diagnostic. The mutation therefore establishes removal sensitivity.
- **L5516–5518:** workflow performs copy-back, byte comparison and source touch.
- **L6696, L6752:** restored source rebuild completes and the focused scene starts.
- **L6759–6760:** restored code fails on CID custody.
- **L6782:** restore step exits **100**, causing the job failure.

The initial diagnostic gate at workflow `:172` worked. The job is red because **restoration did not produce green**, not because the intended mutation went undetected.

## Production cause and precise fixes

### 1. AC5 loses its CID proof during its own successful `podman rm`

Source chain:

- T3 supplies `--cidfile=<spawn>/cid`:  
  `/home/lunarpulse/dev_ws/crates/maos-kernel-core/src/security/sandbox/t3/argv.rs:75–76`.
- Inventory retains only custody **path + spawn boolean**, not the validated CID:  
  `/home/lunarpulse/dev_ws/crates/maos-bin/src/purge_runtime.rs:62,484–508`.
- Containers are removed **before** custody reclamation: `purge_runtime.rs:204–233`.
- Reclamation then opens `<spawn>/cid` for the first time; a missing file immediately means “retain”: `purge_runtime.rs:521–528`.

**Podman removes the supplied CID file when removing its container.** This is an explicit [Podman contract](https://raw.githubusercontent.com/containers/podman/v4.9.3/docs/source/markdown/options/cidfile.write.md), implemented by [`runtime_ctr.go:994–999`](https://github.com/containers/podman/blob/v4.9.3/libpod/runtime_ctr.go#L994-L999).

Consequently, successful container removal destroys the evidence the subsequent custody pass requires. **[INFERENCE] This documented behavior is the cause of these surviving directories; the hosted witness does not print the post-removal directory contents or captured purge output, so the missing `cid` itself is not directly logged.** The production ordering defect exists independently of that remaining observational gap.

**Fix:** at `purge_runtime.rs:484–508`, capture each valid spawn CID **before any runtime removal**, together with validated directory identity/held-directory proof. At `:511–540`, reclaim that same directory using the captured CID and a fresh bounded `container exists == 1` proof; retain no-follow/ownership/inode checks. Keep truly missing/invalid-at-inventory custody conservative. **Do not interpret a missing CID file alone as permission to delete.**

**Witness gap:** `/home/lunarpulse/dev_ws/crates/maos-bin/tests/purge_residue_20_10.rs:63` models `rm` by touching a marker and **never deletes the CID file**. Add a regression where removal deletes the corresponding cidfile before returning absence. The existing real witness is detecting the missing runtime semantic correctly.

For actionable future failures, expand `root_crash.rs:280` diagnostics to include the captured purge stdout/stderr and receipt contents; they are currently discarded on this assertion.

### 2. Separate latent workflow defect in restored-green checking

`/home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/ci/ci-20-10-verify.yml:189` rejects **any nonzero “skipped” count**, while `:188` deliberately selects one of 34 tests. The existing restored log already shows:

- **L6752:** `Starting 1 test … (33 tests skipped)`.
- **L6773:** summary includes `33 skipped`.

Even after production becomes green, the raw grep will reject these legitimate filter exclusions. **This is not the currently exercised failure:** the preceding pipeline exits 100 first.

**Fix:** replace `:189` with the existing `check-log --exit-code 0 --expected-count 1 --required-test root_crash::root_crash_retains_stopped_container_and_purge_reports_it evidence/restored.log`. Its implementation explicitly permits filter-excluded nextest skips while requiring the selected test’s PASS (`scripts/run_cgroup_ceiling_17_2.py:62–69,83–93`).

## Requested category checks

| Suspect | Finding from this run |
|---|---|
| Podman inspect mount-source form | **Not the failing mechanism.** Both owned CIDs disappear and foreign/impostor CIDs survive before the custody assertion. Raw inspect payloads are not logged, so this does not certify every possible source-path form |
| Label filters | **No demonstrated defect.** Production uses one candidate label filter, then checks both labels/name/mount (`purge_runtime.rs:68–127`); real attribution/removal succeeds |
| `rm` / `exists` semantics | Container removal and `exists` codes work in these scenes. **The overlooked `rm` side effect is cidfile deletion**, breaking subsequent custody proof |
| Cgroup creator proof under systemd delegation | **Passes on both heads**, including actual kernel naming, live-creator retention, populated retention and dead-empty removal |
| Custody PID proof / unit PID namespace | **Not this failure.** The failed path is a **spawn child**. Its reclamation branch never consults `process_absent`; PID proof applies only to the process-root branch (`purge_runtime.rs:515–517`). The launcher requests delegation but no private PID namespace (`scripts/run_cgroup_ceiling_17_2.py:404–410`). Do not weaken creator proof |
| Receipt / exit convention | **No demonstrated defect.** Its assertion passed before custody failed; hermetic convention/residue tests also passed. The real witness does not inspect the receipt’s complete contents |

## Main discipline run

At the last query, run **41803503598** was unfinished: both `ci-test-producer` jobs remained **in progress**, and **no `epic-17-exit` jobs were yet listed**. Therefore it had **no finished root_crash verdict to report**.

Read-only investigation; no edits, cargo, test reruns or git mutations performed.