# Development VM cold recovery

`python3 tools/devctl.py vm snapshot --name baseline --json` requests a graceful
ACPI shutdown if the owned VM is running. Copying begins only after QEMU/control
artifacts are gone and kernel exclusive write leases prove both the standalone
qcow2 disk and writable UEFI NVRAM are unused. A lease break aborts the operation;
unsupported leases fail closed. Snapshots are private under
`.local/vm/snapshots/`, contain sparse disk/NVRAM copies, configuration and a
checksum manifest, and have a separate host checksum receipt.

`python3 tools/devctl.py vm restore --name baseline --discard-guest-changes --json`
explicitly discards guest changes since that snapshot. Restore verifies the
manifest, every artifact checksum, enrolled console-rooted target/configuration
and original project-owned disk identity before touching the disk. It preserves
host source, keys, enrollment and reports. The destination inode is retained.
A durable pending marker blocks VM start after an interrupted write; rerunning
the same explicitly authorized restore completes recovery. Different snapshot
requests are refused while recovery is pending.

Restore works without guest SSH. The saved enrollment and project disk identity
bind offline recovery; QMP still verifies the exact process, peer, UUID and disk
for power control. After boot, run `doctor --guest` to verify live pinned SSH and
installed identity before guest execution. Official installer media and registered
serial/QMP operations provide a separate offline inspection/recovery route.

Every restore writes a fresh host recovery epoch outside the reverted disk and
records that approvals were not revalidated. Product policy must consume recovery
epochs and prove approval/task-grant expiry before full recovery acceptance; the
receipt alone is not that proof. Do not restore a live image or manually remove
an interrupted-restore marker.
