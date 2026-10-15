import argparse
import importlib.util
from pathlib import Path
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


class CheckedExecutionTests(unittest.TestCase):
    def setUp(self):
        self.check = load_helper().check_test_output

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
