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
These providers do not establish full release, active filesystem/syscall denial,
restart/load-pressure, quality or performance acceptance.

```fish
python3 tools/devctl.py --workspace /mnt/Storage/PATH_TO_ENROLLED_DEV_WORKSPACE test --suite integration --provider installed-model --detach --json
python3 tools/devctl.py --workspace /mnt/Storage/PATH_TO_ENROLLED_DEV_WORKSPACE test --suite integration --provider installed-model-users --peer-workspace /mnt/Storage/PATH_TO_ENROLLED_TESTER_WORKSPACE --json
python3 tools/devctl.py --workspace /mnt/Storage/PATH_TO_ENROLLED_DEV_WORKSPACE test --suite integration --provider installed-model-idle --detach --json
```

Replace workspace placeholders with independently verified configurations.
The two-user check rejects detached operation so reconnecting cannot substitute
for the original authenticated request owners.
