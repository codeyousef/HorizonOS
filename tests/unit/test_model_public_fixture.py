"""Boundary fixtures only; installed public crash/hash failures run in the VM."""
from pathlib import Path
import sys
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools/guest'))
import model_public_fixture as fixture
import model_lifecycle_preflight as root_fixture


class PublicFailureBoundaryTests(unittest.TestCase):
    def test_nonroot_cannot_enter_coordinator_or_corrupt_mount_control(self):
        for real, effective in ((1000, 1000), (0, 1000), (1000, 0)):
            target = Mock()
            with patch.object(fixture.os, 'getuid', return_value=real), patch.object(fixture.os, 'geteuid', return_value=effective), \
                    patch.object(fixture.os, 'fork') as fork, patch.object(fixture.pwd, 'getpwnam') as account:
                with self.assertRaises(PermissionError): fixture.PublicProbe({}, target)
                with self.assertRaises(PermissionError): root_fixture.corrupt_model({}, Mock())
                target.assert_not_called(); account.assert_not_called(); fork.assert_not_called()

    def test_failure_requires_exact_code_terminal_state_and_no_fabricated_output(self):
        base = {'upstream_exit': 1, 'answer': {'state': 'failed', 'error': 'MODEL_CRASHED', 'output': None, 'mutation_performed': False}}
        fixture.require_failure(base, 'MODEL_CRASHED')
        for key, value in (('state', 'completed'), ('error', 'MODEL_UNAVAILABLE'), ('output', {'text': 'fabricated answer'}), ('mutation_performed', True)):
            with self.assertRaises(RuntimeError): fixture.require_failure({**base, 'answer': {**base['answer'], key: value}}, 'MODEL_CRASHED')
        with self.assertRaises(RuntimeError): fixture.require_failure({**base, 'upstream_exit': 0}, 'MODEL_CRASHED')


if __name__ == '__main__':
    unittest.main()
