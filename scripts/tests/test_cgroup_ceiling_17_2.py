import argparse
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest import mock


HELPER = Path(__file__).resolve().parents[1] / "run_cgroup_ceiling_17_2.py"


def load_helper():
    spec = importlib.util.spec_from_file_location("cgroup_ceiling_17_2", HELPER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def suite(passed, failed=0, ignored=0):
    result = "FAILED" if failed else "ok"
    return (
        f"test result: {result}. {passed} passed; {failed} failed; "
        f"{ignored} ignored; 0 measured; 0 filtered out; finished in 0.01s\n"
    )


def nextest_suite(total, outcomes=None, *, summary=True):
    prefix = "     Summary [   0.010s] " if summary else ""
    if outcomes is None:
        outcomes = f"{total} passed"
    noun = "test" if total == 1 else "tests"
    return f"{prefix}{total} {noun} run: {outcomes}\n"


def nextest_no_capture(names=("t3::first", "t3::second", "t3::third"), outcomes=None):
    """One child libtest summary per process, then nextest's aggregate."""
    output = f"Starting {len(names)} tests across 1 binary (4 skipped)\n"
    for index, name in enumerate(names, start=1):
        output += (
            f"       START [         ] ({index}/{len(names)}) "
            f"maos-bin::cgroup_ceiling_smoke {name}\n"
            f"\nrunning 1 test\ntest {name} ... fixture diagnostic\nok\n"
            + suite(1)
            + f"        PASS [   0.001s] ({index}/{len(names)}) "
            f"maos-bin::cgroup_ceiling_smoke {name}\n"
        )
    return output + nextest_suite(len(names), outcomes or f"{len(names)} passed, 4 skipped")


class CheckedExecutionTests(unittest.TestCase):
    def setUp(self):
        self.check = load_helper().check_test_output

    def test_nextest_plain_and_summary_output_are_accepted(self):
        for summary in (False, True):
            with self.subTest(summary=summary):
                self.check(nextest_suite(22, summary=summary), 0, [22])
                self.check(nextest_suite(1, summary=summary), 0, [1])

    def test_nextest_reported_skips_are_outside_the_run_count(self):
        self.check(nextest_suite(22, "22 passed, 103 skipped"), 0, [22])
        self.check(nextest_suite(22, "22 passed (1 slow), 103 skipped"), 0, [22])
        self.check(nextest_suite(22, "22 passed, 0 skipped"), 0, [22])

    def test_nextest_total_and_passed_must_both_match_expected_count(self):
        for total, outcomes in (
            (21, "21 passed"),
            (23, "23 passed"),
            (23, "22 passed"),
            (22, "21 passed"),
            (21, "22 passed"),
        ):
            with self.subTest(total=total, outcomes=outcomes), self.assertRaises(ValueError):
                self.check(nextest_suite(total, outcomes), 0, [22])

    def test_nextest_failed_timed_out_ignored_or_incomplete_suites_are_rejected(self):
        for outcomes in (
            "21 passed, 1 failed",
            "22 failed",
            "21 passed, 1 timed out",
            "21 passed, 1 ignored",
            "22 passed, 1 failed",
            "22 passed, 1 ignored",
            "21 passed, 1 cancelled",
            "21 passed, 1 not run",
        ):
            with self.subTest(outcomes=outcomes), self.assertRaises(ValueError):
                self.check(nextest_suite(22, outcomes), 0, [22])

    def test_nextest_zero_tests_or_zero_expected_count_are_rejected(self):
        for text, count in (
            (nextest_suite(0), 22),
            (nextest_suite(0, "0 passed, 22 skipped"), 22),
            (nextest_suite(0), 0),
            (nextest_suite(22), 0),
        ):
            with self.subTest(text=text, count=count), self.assertRaises(ValueError):
                self.check(text, 0, [count])

    def test_nextest_failed_command_is_not_hidden_by_a_successful_summary(self):
        with self.assertRaises(ValueError):
            self.check(nextest_suite(22), 100, [22])

    def test_nextest_missing_extra_or_reordered_summaries_are_rejected(self):
        for text, counts in (
            ("Starting 22 tests across 1 binary\n", [22]),
            (nextest_suite(22) + nextest_suite(0), [22]),
            (nextest_suite(22) + nextest_suite(4), [4, 22]),
            (nextest_suite(22) + nextest_suite(1, "1 failed"), [22]),
        ):
            with self.subTest(text=text, counts=counts), self.assertRaises(ValueError):
                self.check(text, 0, counts)
        self.check(nextest_suite(4) + nextest_suite(22), 0, [4, 22])
        self.check(suite(4) + nextest_suite(22), 0, [22])
        self.check(nextest_suite(4) + suite(22), 0, [4])
        for text in (suite(4) + nextest_suite(22), nextest_suite(4) + suite(22)):
            with self.subTest(text=text), self.assertRaises(ValueError):
                self.check(text, 0, [4, 22])

    def test_no_capture_uses_exact_aggregate_not_each_child_summary(self):
        output = nextest_no_capture()
        self.check(output, 0, [3], ["t3::first", "t3::second", "t3::third"])
        for counts in ([1], [1, 1, 1, 3], [2], [4]):
            with self.subTest(counts=counts), self.assertRaises(ValueError):
                self.check(output, 0, counts)
        canary = "maos_run_cli_wrapper_worker_spawns_real_subprocess"
        self.check(nextest_no_capture((canary,)), 0, [1], [canary])

    def test_no_capture_preserves_ordered_multiple_aggregate_counts(self):
        output = nextest_no_capture(("t3::first",)) + nextest_no_capture()
        self.check(output, 0, [1, 3], ["t3::first", "t3::second", "t3::third"])
        for counts in ([3, 1], [1], [1, 1, 3]):
            with self.subTest(counts=counts), self.assertRaises(ValueError):
                self.check(output, 0, counts)
        partial = output.rsplit("     Summary", 1)[0]
        with self.assertRaises(ValueError):
            self.check(partial, 0, [1, 3])

    def test_no_capture_child_success_cannot_hide_bad_aggregate(self):
        output = nextest_no_capture()
        for summary in (
            nextest_suite(0, "0 passed, 7 skipped"),
            nextest_suite(3, "2 passed, 1 failed, 4 skipped"),
            nextest_suite(3, "2 passed, 1 timed out, 4 skipped"),
            nextest_suite(3, "2 passed, 1 not run, 4 skipped"),
            nextest_suite(3, "2 passed, 4 skipped"),
            nextest_suite(2, "2 passed, 4 skipped"),
        ):
            with self.subTest(summary=summary), self.assertRaises(ValueError):
                self.check(output.rsplit("     Summary", 1)[0] + summary, 0, [3])
        with self.assertRaises(ValueError):
            self.check(output, 100, [3])

    def test_no_capture_requires_pass_names_not_libtest_headers(self):
        output = nextest_no_capture()
        pass_line = (
            "        PASS [   0.001s] (2/3) "
            "maos-bin::cgroup_ceiling_smoke t3::second\n"
        )
        for replacement in ("", pass_line.replace("t3::second", "t3::unrelated")):
            with self.subTest(replacement=replacement), self.assertRaisesRegex(
                ValueError, "t3::second"
            ):
                self.check(output.replace(pass_line, replacement), 0, [3], ["t3::second"])

    def test_fixed_suite_replacements_cannot_satisfy_original_required_names(self):
        cases = (
            ("worker_resource_limits_17_2", (
                "real_maos_worker_applies_independent_t3_limits_and_observes_exhaustion",
                "fd_request_above_inherited_hard_limit_refuses_before_worker_payload",
            )),
            ("spawned_scope_17_3c", (
                "eight_real_admitted_forms_use_trusted_sender_policy_not_echoed_claims",
                "forged_guest_provenance_is_restamped_by_the_kernel",
                "a_delivery_needs_its_iac_send_grant",
                "wasm_delivery_is_refused_when_its_issuance_window_is_exhausted",
                "kernel_only_cross_host_oversized_and_foreign_retract_frames_are_refused",
                "guest_floods_end_in_typed_failures_not_a_wedged_session",
                "guest_diagnostics_are_bounded_per_session",
                "runner_refusal_exits_before_ready_are_typed_and_not_crashes",
                "a_once_session_that_never_completes_its_turn_is_not_success",
                "hostile_metadata_fails_before_serialization_under_real_t2",
                "hostile_aliased_strings_fail_before_serialization_under_real_t2",
                "one_aliased_string_under_the_hostcall_budget_clears_the_canonical_call",
                "actual_runner_sigsys_reaches_escape_consumer_but_planned_stop_does_not",
                "repeated_mailbox_exports_survive_idle_without_an_active_task_or_stall",
                "a_new_turn_after_idle_restarts_the_progress_window",
                "configured_enterprise_and_mixed_scope_refusals_are_atomic_for_both_forms",
                "resource_downgrade_memory_only_is_persisted_for_each_fast_t2_restart",
                "memory_only_t2_live_child_has_exact_rlimits_and_mode_truthful_journal",
                "resource_downgrade_no_resource_or_fd_only_t2_spawn_emits_no_row",
                "resource_downgrade_strict_cpu_pid_and_failed_t2_spawns_emit_no_row",
                "resource_downgrade_append_panic_follows_live_child_observer_installation",
                "admitted_t2_echo_enforces_exact_live_caps_in_delegated_mode",
                "fast_t2_crash_then_restart_orders_one_downgrade_after_crash",
            )),
        )
        for binary, required in cases:
            count = len(required)
            output = nextest_no_capture(required).replace(
                "maos-bin::cgroup_ceiling_smoke", f"maos-bin::{binary}"
            )
            self.check(output, 0, [count], required)
            for missing in required:
                with self.subTest(binary=binary, missing=missing):
                    # A passing replacement preserves the aggregate; nextest's
                    # skipped count does not identify an ignored original case.
                    replacement = output.replace(missing, missing + "_replacement")
                    self.check(replacement, 0, [count])
                    with self.assertRaisesRegex(ValueError, missing):
                        self.check(replacement, 0, [count], required)

    def test_no_capture_still_refuses_explicit_runtime_skip(self):
        for skip in ("SKIP rootless runtime unavailable\n", "test t3::first ... SKIP unavailable\n"):
            with self.subTest(skip=skip), self.assertRaisesRegex(ValueError, "SKIP"):
                self.check(nextest_no_capture() + skip, 0, [3])

    def test_ansi_color_does_not_hide_summaries_or_explicit_skips(self):
        for text in (
            nextest_suite(22).replace("Summary", "\x1b[1;32mSummary\x1b[0m").replace(
                "22 passed", "\x1b[32m22 passed\x1b[0m"
            ),
            "\x1b[32m" + suite(22) + "\x1b[0m",
        ):
            with self.subTest(text=text):
                self.check(text, 0, [22])
        for text in (
            "\x1b[33mSKIP\x1b[0m missing rootless runtime\n",
            "test t3::worker_limits ... \x1b[33mSKIP\x1b[0m substrate not provisioned\n",
        ):
            with self.subTest(text=text), self.assertRaisesRegex(ValueError, "SKIP"):
                self.check(text + nextest_suite(22), 0, [22])

    def test_nextest_required_tests_must_have_pass_records(self):
        output = (
            "        PASS [   0.001s] ( 1/2) maos-bin::cgroup_ceiling_smoke t3::first\n"
            "        \x1b[32mPASS\x1b[0m [   0.002s] (2/2) "
            "maos-bin::cgroup_ceiling_smoke t3::second\n"
            + nextest_suite(2)
        )
        self.check(output, 0, [2], ["t3::first", "t3::second"])
        self.check(output.replace("( 1/2) ", "").replace("(2/2) ", ""),
                   0, [2], ["t3::first", "t3::second"])
        for replacement in ("unrelated", "t3::second_extra"):
            with self.subTest(replacement=replacement), self.assertRaisesRegex(
                ValueError, "t3::second"
            ):
                self.check(output.replace("t3::second", replacement), 0, [2],
                           ["t3::first", "t3::second"])
        for status in ("FAIL", "SKIP", "START"):
            with self.subTest(status=status), self.assertRaises(ValueError):
                self.check(output.replace("\x1b[32mPASS", "\x1b[32m" + status),
                           0, [2], ["t3::first", "t3::second"])

    def test_libtest_filtered_out_cases_remain_accepted(self):
        self.check(suite(22).replace("0 filtered out", "103 filtered out"), 0, [22])

    def test_libtest_ansi_color_does_not_hide_failed_or_ignored_assertions(self):
        for text in (suite(21, 1), suite(22, ignored=1)):
            with self.subTest(text=text), self.assertRaises(ValueError):
                self.check("\x1b[32m" + text + "\x1b[0m", 0, [22])

    def test_zero_selected_cases_are_not_success(self):
        with self.assertRaises(ValueError):
            self.check(suite(0), 0, [22])

    def test_failed_or_ignored_required_assertions_are_not_success(self):
        for text in (suite(21, 1), suite(21, ignored=1)):
            with self.subTest(text=text), self.assertRaises(ValueError):
                self.check(text, 0, [22])

    def test_failed_command_is_not_hidden_by_a_successful_test_summary(self):
        with self.assertRaises(ValueError):
            self.check(suite(22), 101, [22])

    def test_missing_extra_or_incomplete_required_suite_is_rejected(self):
        for text in ("", suite(21), suite(22) + suite(0), suite(4)):
            with self.subTest(text=text), self.assertRaises(ValueError):
                self.check(text, 0, [22])

    def test_explicit_substrate_skip_is_not_counted_as_execution(self):
        # Serial `--nocapture` libtest prints `test <name> ... ` before the body,
        # so the test's own SKIP line lands after that prefix.
        for text in (
            "SKIP missing rootless runtime\n",
            "test t3::worker_limits ... SKIP T3 worker_limits: substrate not provisioned\nok\n",
        ):
            with self.subTest(text=text), self.assertRaises(ValueError):
                self.check(text + suite(2), 0, [2])

    def test_all_required_suites_must_complete_in_order(self):
        self.check(suite(4) + suite(22), 0, [4, 22])
        with self.assertRaises(ValueError):
            self.check(suite(22) + suite(4), 0, [4, 22])

    def test_passing_replacement_cannot_substitute_for_a_required_scene(self):
        output = "test s1 ... fixture diagnostic\nok\ntest s2 ... ok\n" + suite(2)
        self.check(output, 0, [2], ["s1", "s2"])
        replacement = output.replace("test s2 ...", "test unrelated ...")
        with self.assertRaisesRegex(ValueError, "s2"):
            self.check(replacement, 0, [2], ["s1", "s2"])
        with self.assertRaisesRegex(ValueError, "s2"):
            self.check(output.replace("test s2 ... ok\n", ""), 0, [2], ["s1", "s2"])


class EnvironmentTests(unittest.TestCase):
    def test_archive_guards_fixture_and_locations_survive_the_cgroup_envelope(self):
        helper = load_helper()
        caller = {
            "PATH": "/usr/bin",
            "HOME": "/home/runner",
            "RUSTC": "/bin/false",
            "CARGO": "/bin/false",
            "CARGO_BUILD_RUSTC": "/bin/false",
            "MAOS_FIXTURE_DIR": "/workspace/target/maos-fixtures",
            "MAOS_FIXTURE_TRANSPORT": "1",
            "MAOS_CGROUP_CEILING_FIXTURE": "/workspace/target/tmp/cgroup-ceiling-fixture",
            "MAOS_CGROUP_CANARY_ARCHIVE": "/runner/_temp/ci-test-archives/maos-bin-network-wasm.tar.zst",
            "RUNNER_TEMP": "/runner/_temp",
            "GITHUB_WORKSPACE": "/workspace",
        }
        for mode in ("delegated", "undelegated"):
            with self.subTest(mode=mode), mock.patch.dict(
                helper.os.environ, {**caller, "UNRELATED_SECRET": "not transported"}, clear=True
            ):
                self.assertEqual(
                    helper._environment(mode), {**caller, "MAOS_17_2_CGROUP_MODE": mode}
                )

    def test_local_environment_does_not_invent_archive_overrides(self):
        helper = load_helper()
        caller = {"PATH": "/usr/bin", "HOME": "/home/developer"}
        with mock.patch.dict(helper.os.environ, caller, clear=True):
            self.assertEqual(
                helper._environment("delegated"),
                {**caller, "MAOS_17_2_CGROUP_MODE": "delegated"},
            )


class CanaryArchiveBoundaryTests(unittest.TestCase):
    def test_compiler_denial_requires_explicit_archive_before_execution(self):
        helper = load_helper()
        for key in ("CARGO", "RUSTC", "CARGO_BUILD_RUSTC"):
            for denial in ("/bin/false", "/usr/bin/false", "false"):
                with self.subTest(key=key, denial=denial), mock.patch.dict(
                    helper.os.environ, {key: denial}, clear=True
                ), self.assertRaisesRegex(RuntimeError, "compiler-denied.*MAOS_CGROUP_CANARY_ARCHIVE"):
                    helper._rootless_canary_command()

    def test_configured_empty_relative_missing_or_nonfile_archive_is_refused(self):
        helper = load_helper()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            broken = root / "broken.tar.zst"
            broken.symlink_to(root / "missing.tar.zst")
            for archive in ("", "relative.tar.zst", str(root / "missing.tar.zst"), str(root), str(broken)):
                for compiler_denied in (False, True):
                    caller = {"MAOS_CGROUP_CANARY_ARCHIVE": archive, "GITHUB_WORKSPACE": directory}
                    if compiler_denied:
                        caller["RUSTC"] = "/bin/false"
                    with self.subTest(archive=archive, compiler_denied=compiler_denied), mock.patch.dict(
                        helper.os.environ, caller, clear=True
                    ), self.assertRaisesRegex(RuntimeError, "MAOS_CGROUP_CANARY_ARCHIVE"):
                        helper._rootless_canary_command()

    def test_archive_requires_real_absolute_workspace_before_execution(self):
        helper = load_helper()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "archive.tar.zst"
            archive.touch()
            for workspace in (None, "", "relative", str(root / "missing"), str(archive)):
                caller = {"MAOS_CGROUP_CANARY_ARCHIVE": str(archive)}
                if workspace is not None:
                    caller["GITHUB_WORKSPACE"] = workspace
                with self.subTest(workspace=workspace), mock.patch.dict(
                    helper.os.environ, caller, clear=True
                ), self.assertRaisesRegex(RuntimeError, "GITHUB_WORKSPACE"):
                    helper._rootless_canary_command()


class UnitNameTests(unittest.TestCase):
    def test_unit_type_suffixed_names_are_refused_before_launch(self):
        helper = load_helper()
        for unit in ("maos-17-2-x.service", "maos-17-2-x.scope", "maos-17-2-x@y",
                     "maos-17-7-terminal.service", "maos-17-5-terminal", "other-17-7-x"):
            args = argparse.Namespace(
                unit=unit, mode="delegated", timeout=5, rootless_canary=False, command=["true"],
            )
            with self.subTest(unit=unit), \
                    mock.patch.object(helper.os, "geteuid", return_value=1000), \
                    mock.patch.object(helper.subprocess, "Popen") as popen, \
                    self.assertRaisesRegex(ValueError, "unit"):
                helper._systemd_run(args)
            popen.assert_not_called()


if __name__ == "__main__":
    unittest.main()
