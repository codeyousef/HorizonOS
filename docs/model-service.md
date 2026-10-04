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
The reusable NixOS module enables this service with
`services.aios.model.enable = true` and an exact
`services.aios.model.manifest` from the supplied `aiosModelArtifact` package.
Composition supplies the reviewed `aiosModel` and `aiosModelArtifact` packages
through module arguments. `services.aios.users` lists existing normal users
allowed into `aios-inference`; unknown, duplicate or system users are rejected.
The unit is discovered through the package's `lib/systemd/system` directory,
and only its socket is wanted during boot. Full control-plane enablement is
still guarded until its services are implemented.

The module creates `/etc/aios/model-runtime.json` as a root-owned immutable
store link. The daemon accepts only this fixed configuration path, validates
its strict schema and file identity before startup, and exposes no configuration
operation to clients. Network access must remain false; low/high profiles and
other context sizes fail closed until their artifacts/runtime are qualified.
`model.threads` is null by default, reserving a CPU where possible and choosing
at most four; explicit values must fit both the four-thread limit and the
available CPUs. `model.idleUnloadSeconds` defaults to 600. Zero disables only
automatic unload; explicit unload, deadlines, authentication and quotas remain.

Generate the implemented options' documentation with
`nix build --no-update-lock-file --no-write-lock-file .#lib.modelOptionsDocumentation`
in a verified guest. Module evaluation and effective installed-unit isolation
are separate checks; an evaluation does not prove a running sandbox.

`aios-model-artifact` is the independent normal-profile data package shared by
desktop, headless and recovery image composition. It uses fixed hashes and
`requireFile`, with no fetcher, inference-code dependency or conversion build.
An administrator imports the reviewed GGUF and original metadata once with
`nix-store --add-fixed sha256`; building code reuses that store data. The package
contains read-only weights, exact source/conversion locks and original
tokenizer/config/template/license metadata, without original training weights,
Python or PyTorch. Select this package as the image's model directory and retain
the previous package until the new artifact passes acceptance.

Before native loading, Rust verifies the GGUF digest, complete source provenance,
metadata bytes and the pinned converter and quantizer binaries. Production
requires exact packaged manifests and root-owned immutable store files;
development qualification requires a strictly parsed matching conversion
receipt and owned read-only files. Original weight hashes bind the reviewed
conversion to its exact output; inference does not need the original source
weights. The embedded chat template is also checked after opening the model.
Missing, changed, writable, linked or redirected development inputs fail closed.

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
Production startup then performs thirteen fixed kernel denial checks: `execve`
and `execveat`, IPv4/IPv6 socket creation, opening home/root/user-runtime/log/
system-bus/Nix-daemon directories, and write-opening the immutable model manifest
and runtime configuration. No directory entries, protected contents, or model
bytes are read or modified by these checks. Unexpected success, a missing path,
or an unrelated error prevents startup. Status includes only the fixed boundary
names and denial errno values, without caller-selected paths or prompts.
Status reports coarse lifecycle/budget metadata and the caller's queued count.
Unload refuses to interrupt active or queued work; idle unload is scheduled
after the configured interval (600 seconds by default) without inference. The low/high profiles report unavailable
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

## Initial model-enabled acceptance image

The `aios-model-test` image installs the real socket-activated model service for
both `dev` and `tester`. It inherits the disposable desktop's synthetic login;
this image is not a production release. The other development/desktop images
remain model-disabled.

The host administrator exports public data from an enrolled builder's immutable
`horizon-os-model-normal-*` package with:

```fish
python3 tools/devctl.py model-seed --artifact /nix/store/REVIEWED_MODEL_DATA_PACKAGE --json
python3 tools/devctl.py test --suite desktop --with-model --json
```

Use the actual package path reported by the verified model-artifact build.
The export admits only the exact locked weights and small source metadata,
checks pinned guest identity before and after each transfer, and verifies size
and SHA256 before accepting a file. All persistent host artifacts live under
`/mnt/Storage`. The private `.local/model-seed` cache is data, never a Nix source
or inference download route. Partial or changed caches fail closed.

Initial provisioning copies the data into read-only seed media. The registered
installer validates every data checksum/size before formatting its authorized fresh virtual disk, then repeats those checks
and imports the fixed files into the mounted target store used by nixos-install. The image's existing
`requireFile` derivation then creates an independent immutable data package.
Seed media is removed before installed startup. This route cannot activate or
replace an existing installation; guarded deployment remains a separate
contract. Creating/booting this image alone does not establish inference,
sandbox, two-user privacy, or elapsed idle-unload acceptance.

After enrolling the installed image, its registered `installed-model` provider
checks the actual root-managed socket, production-mode executable, immutable
runtime/model data, effective systemd limits and process privileges. It uses
the installed daemon for cited answers, request ownership, quotas, deadlines,
cancellation, overflow and explicit unload; it does not launch a replacement.
It also checks rejection of tool execution, model download/load routes,
caller-supplied authority and custom grammars, and permits only lifecycle/budget
metadata in status and the fixed acknowledgement in unload responses.
`installed-model-users` requires two separately enrolled workspaces for different
normal users on that same VM. It retains both original connections, verifies
foreign result/cancel denial and serialization, and checks that an owner-only
synthetic private marker is absent from the other user's answer. Target identity
is checked again during coordination, and failed attempts retain evidence.

The `installed-model-idle` provider measures the configured 600-second interval
after a successful generation, polling coarse status without resetting the idle
clock. It checks that the same service process unloads its weights and records
cgroup memory samples. Run it exclusively: other generation/unload operations
or a VM restart invalidate the measurement. Cgroup memory is not process PSS.
These providers do not establish full release, restart/load-pressure, quality or
performance acceptance.

The model acceptance image also runs one fixed initial root fixture, with no RPC,
arguments or sudo route. It verifies the immutable target authority before each
operation, records process PSS samples, kills only `aios-model.service` during a
real request, measures four increasing restart delays, checks that private
request state was discarded, and asks the recovered service for a cited answer.
Its test-only capability exceptions are `CAP_KILL` and `CAP_SYS_PTRACE` for the
fixed kill and process observation, and `CAP_SETUID`/`CAP_SETGID` to permanently
drop forked clients to five fixed normal UIDs before they connect. The extra
three accounts have locked passwords, no login shell, no home and no SSH keys.
The module asserts a development image and
is absent from production composition. The production model gains no privilege.
The fixture compares the root-owned read-only installed authority file with the
immutable compiled authority in the current system closure, then checks the
actual installation, DMI, disk and management identities. Its execution does not
hold the multi-user or graphical boot targets while measurements run.

The fixed load coordinator uses bounded JSON over private socket pairs. It
closes inherited privileged descriptors, checks actual process credentials and
zero effective/permitted/inheritable/ambient capabilities, and fills the real
CPU daemon with one running and eight waiting requests. A long public synthetic
prefill retains the worker without a test hook or paused daemon. It rejects the
next request while that UID is below its own quota, verifies foreign UID denial,
then broadcasts cancellation and requires all nine contexts to finish within
two seconds. It records a process PSS sample at full queue; this is not a peak
measurement. Only its own unreaped children can be signalled during cleanup.

The fixture's first crash is a real public `aiosctl ask` through the unchanged
installed broker/unit as a permanently dropped normal tester. It requires a
terminal `MODEL_CRASHED` with no output or mutation. Fixed deterministic SSH
inspection and KWin's read-only `supportInformation` must still respond; the
desktop processes must retain their identities. This fixture binds only the
tester runtime under its empty home view, and orders itself after that user's
manager without holding the graphical target.

For hash-mismatch acceptance, a separate immutable test artifact has exactly one
changed GGUF byte and the original length. The root fixture can create only its
fixed root-owned runtime drop-in in `/run/systemd/system/aios-model.service.d`.
It adds a read-only bind of that file over the original GGUF in the same
production-mode daemon's mount view, observes the actual namespace digest,
requires public `TARGET_CHANGED` with no answer and no retained loaded state,
then removes only the exact owned drop-in and restores/restarts the original
view. No store data is patched, no model capabilities are added, and there is
no public root configuration route. Deterministic service/KWin responses and a
real cited answer after restoration are required.

The root-owned report binds the current installation and boot. The normal-user
`installed-model-lifecycle` provider reads it without mutations. Desktop acceptance
waits for the same initial fixture; failure retains the VM and evidence. PSS
observations are samples rather than a true peak. During pending tests, a
separate protected phase file and fresh model-unit state let the registered
host runner record actual pinned SSH/desktop reads while inference is restarting
or corrupt. It requires observations in both failure phases. These finite
samples do not establish continuous availability or latency qualification.

```fish
python3 tools/devctl.py --workspace /mnt/Storage/PATH_TO_ENROLLED_DEV_WORKSPACE test --suite integration --provider installed-model --detach --json
python3 tools/devctl.py --workspace /mnt/Storage/PATH_TO_ENROLLED_DEV_WORKSPACE test --suite integration --provider installed-model-users --peer-workspace /mnt/Storage/PATH_TO_ENROLLED_TESTER_WORKSPACE --json
python3 tools/devctl.py --workspace /mnt/Storage/PATH_TO_ENROLLED_DEV_WORKSPACE test --suite integration --provider installed-model-idle --detach --json
python3 tools/devctl.py --workspace /mnt/Storage/PATH_TO_ENROLLED_DEV_WORKSPACE test --suite integration --provider installed-model-lifecycle --detach --json
python3 tools/devctl.py --workspace /mnt/Storage/PATH_TO_ENROLLED_DEV_WORKSPACE test --suite integration --provider installed-session-inference --detach --json
```

Replace workspace placeholders with independently verified configurations.
The two-user check rejects detached operation so reconnecting cannot substitute
for the original authenticated request owners.
