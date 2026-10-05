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

The session broker's typed direct observation route issues private process
handles and immutable snapshot cursors. Both bind the full originating peer
(including connection identity), expire after 30 seconds on a suspend-inclusive
clock and are revoked on private-client disconnect. A cursor also binds the
query limit, snapshot and offset. Every returned process is inspected live;
an exited or changed snapshot member produces an explicit error. Inventories
are capped at 256 retained processes, 32768 proc entries and two seconds;
pages contain at most 100 entries. Quotas bound retained native descriptors.
Unreadable own-user processes produce a `partial` result with a
`PERMISSION_DENIED` explanation, including on the final page. An unreadable
inventory never becomes a complete healthy empty result. Resource exhaustion
and malformed native records return errors rather than truncated success.
Application filters are explicitly unsupported until native application
associations are implemented. Reads check the same policy grant before and
after native work, and reauthenticate the originating peer at both boundaries.
`org.aios.Agent1.ListProcesses` and `InspectProcess` accept strict versioned
requests and fix their action identifiers on the server. They cannot dispatch
other actions or accept a caller-selected UID, PID or privileged namespace.

There is no signal method in the observation library and no termination route.
The model-facing process capabilities remain unavailable until original task
scope integration, consent-bound graceful effects and installed verification
are complete. Broker unit tests use actual proc/pidfd observations and native
caller identity; the two-entry cursor corpus is controlled, not a claim of an
installed inventory qualification.

The native descriptor lifetime follows the Linux
[pidfd documentation](https://man7.org/linux/man-pages/man2/pidfd_open.2.html).
