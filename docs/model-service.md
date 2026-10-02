# Horizon OS inference service

`aios-modeld` accepts inference requests through a local Unix socket. It has no
tool executor, authorization, file inspection, network or download method.
Generated tool calls are proposals that the user broker must independently
validate against its registry, evidence and authenticated scope.

The production entry point requires the `aios-model` identity and one systemd
socket-activation descriptor for `/run/aios/model.sock`. The socket is root/model
owned, group `aios-inference`, mode 0660. The configured model directory resolves
to a root-owned immutable Nix store artifact. Packaged unit templates declare
network/home/log/Nix-socket isolation, resource bounds and restart backoff.
Image/module activation and effective production sandbox verification are
separate deployment acceptance checks.

Connections use kernel peer credentials and the kernel's peer pidfd. Every read
also validates message credentials, so an inherited or transferred descriptor
cannot silently change the authenticated process. Unexpected ancillary file
descriptors are closed and rejected. See the Linux [Unix socket contract](https://man7.org/linux/man-pages/man7/unix.7.html).
Request ownership includes UID, GID, PID and a server-issued connection identity.
Only that connection can read or cancel its results. Closing it discards queued
work/results and cancels active work. Clients poll or send control messages
within the ten-second connection inactivity limit.

The [request schema](../schemas/model-request.schema.json) uses the standard
four-byte big-endian frame length and at most 64 KiB of UTF-8 JSON. Operations
are `generate`, `get_status`, `get_result`, `cancel` and `unload`. Unknown or
duplicate security-sensitive fields are rejected. Clients select a fixed
response mode, permitted read-tool names and evidence references; they cannot
supply a decoding grammar, executable code or approval tokens. The service
builds its grammar and independently parses the completed output.

One worker owns the model and generates at a time. Eight requests may wait;
each UID may have two outstanding requests, eight retained records and four
connections. Total connections are capped at 32 and result records at 64.
Queue time counts toward the requested deadline, capped at 90 seconds. Queued
expiration works independently of active generation. Native cancellation
interrupts hashing/loading, prompt evaluation and decoding. The normal context
is 8192 tokens with at most 6144 input tokens, 192 decision tokens or 768 final
answer tokens. Threads are bounded by the normal profile.

Every generation has a fresh context; private KV/prefix state is cleared on
teardown. Prompt and response buffers are wiped where owned by the wrapper.
Results expire after 30 seconds and ordinary logs contain no prompt text.
The daemon installs an execution-denial seccomp filter before creating threads.
Status reports coarse lifecycle/budget metadata and the caller's queued count.
Unload refuses to interrupt active or queued work; idle unload is scheduled
after 600 seconds without inference. The low/high profiles report unavailable
until their own artifacts and qualification exist, with no automatic fallback.

The explicit development qualification entry point is available only in a
verified development guest, as a non-root user with a private mode-0700 socket
directory. It exercises the actual model, queue, credentials, deadlines,
cancellation and unload. It does not establish production account/isolation,
two-real-user, memory/quality/performance or elapsed-600-second acceptance.

Run the registered guest checks from the host:

```sh
python3 tools/devctl.py test --suite unit --detach --json
python3 tools/devctl.py test --suite integration --provider model-service --detach --json
```
