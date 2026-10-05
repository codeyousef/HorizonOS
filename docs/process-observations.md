# Own-user process observations

`aios-system::processes::OwnProcess` retains a native Linux pidfd and the
original proc directory. Construction captures the current non-root effective
UID, PID, process start ticks, boot ID and executable device/inode/timestamps.
It verifies procfs type, directory ownership and all four native UID fields.
Mixed privileged credentials are refused. Inspection checks the original
identity twice and refuses a changed executable, ownership, boot, start time
or exited process. A reused PID cannot redirect the retained proc directory
or pidfd to a newly selected process.

The observation contains CPU time, resident memory and thread count. Numeric
values fit the protocol's safe integer range. Fixed proc fields are bounded to
64 KiB. Process names, executable paths, command lines, environment values and
arbitrary proc fields are not returned. The executable locator identifies the
native file object; it is not a binary content hash.

The library does not issue wire handles, grant policy scopes or perform
termination. Those remain the authenticated session broker's responsibility.
There is no signal method in this observation API. The model-facing process
capabilities remain unavailable until broker integration, consent-bound
graceful effects and installed verification are complete.

The native descriptor lifetime follows the Linux
[pidfd documentation](https://man7.org/linux/man-pages/man2/pidfd_open.2.html).
