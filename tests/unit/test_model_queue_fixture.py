"""Coordinator boundary checks only; real queue qualification runs in the VM."""
from pathlib import Path
import socket
import struct
import sys
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools/guest'))
import model_queue_fixture as fixture


class QueueCoordinatorBoundaryTests(unittest.TestCase):
    def test_nonroot_denied_before_identity_accounts_or_fork(self):
        for real, effective in ((1000, 1000), (0, 1000), (1000, 0)):
            target, process = Mock(), Mock()
            with patch.object(fixture.os, 'getuid', return_value=real), patch.object(fixture.os, 'geteuid', return_value=effective), \
                    patch.object(fixture.pwd, 'getpwnam') as account, patch.object(fixture.os, 'fork') as fork:
                with self.assertRaises(PermissionError):
                    fixture.run({}, target, process)
                target.assert_not_called(); process.assert_not_called()
                account.assert_not_called(); fork.assert_not_called()

    def frame(self, data, length=None):
        left, right = socket.socketpair()
        self.addCleanup(left.close); self.addCleanup(right.close)
        right.sendall(struct.pack('!I', len(data) if length is None else length) + data)
        right.shutdown(socket.SHUT_WR)
        return left

    def test_frames_fail_closed(self):
        for data in (b'{"kind":1,"kind":2}', b'{"kind":NaN}', b'[]', b'{"kind":'):
            with self.assertRaises(ValueError):
                fixture.receive(self.frame(data))
        for size in (0, fixture.MAX_FRAME + 1, 0xffffffff):
            with self.assertRaises(ValueError):
                fixture.receive(self.frame(b'', size))
        with self.assertRaises(RuntimeError):
            fixture.receive(self.frame(b'{}', 4))

    def test_private_roundtrip_and_outbound_bound(self):
        left, right = socket.socketpair()
        self.addCleanup(left.close); self.addCleanup(right.close)
        fixture.send(right, {'kind': 'snapshot'})
        self.assertEqual(fixture.receive(left), {'kind': 'snapshot'})
        for payload in ({'value': float('nan')}, {'value': 'a' * fixture.MAX_FRAME}):
            with self.assertRaises(ValueError):
                fixture.send(right, payload)


if __name__ == '__main__':
    unittest.main()
