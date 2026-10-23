# Descriptor-only document extraction

`aios-extract` is an unprivileged parsing worker. It grants no root, file,
indexing or model authority. A broker must authorize content access before
transferring its read-only file descriptor and recheck the original grant and
live source identity before using a response. The worker does not enroll paths
or persist parsed content. Documents remain observations, not instructions.

The optional NixOS `services.aios.extract.enable` module installs a private
per-user socket and short-lived service instances. The socket admits one active
connection at a time. Its parent directory is 0700 and its socket is 0600. The
service has separate PID, user, mount and network namespaces, hides homes and
runtime sockets, and permits no IP socket creation. Startup verifies the
restricted view before receiving content. Private scratch is a 64 MiB noexec
filesystem. Neither original source paths nor executable/options/password fields
are part of the request protocol.

The worker first sends `0xe2`. The client enables `SO_PASSCRED` before connecting
and authenticates that byte’s kernel sender credentials against the active
managed worker, its compile-pinned executable and resource restrictions. The
socket listener credentials identify the user manager and are insufficient on
their own. Only after authentication does the client send `0xe1` carrying exactly one kernel
`SCM_RIGHTS` descriptor, then a four-byte big-endian JSON length and that JSON.
The caller closes its write half after the request. The request is limited to
4096 bytes and contains `schema_version: 1`, a `format` (`text`, `pdf` or
`metadata`) and an optional typed `range`. Line/page endpoints are one-based,
inclusive and ordered. Page ranges contain at most 200 pages; line selections
contain at most 100000 lines. Writable, nonregular and foreign-owner descriptors,
extra rights, unknown fields and malformed requests are rejected. A response
contains either typed extraction data or a stable error code. This internal
socket is not a public model tool or an approval transport. The broker implements this handshake with bounded connection/read deadlines
and no retries after an ambiguous transfer.

Automatic extraction has a 50 MiB source limit. Larger sources return metadata
with an explicit limitation. On-demand line/page ranges can inspect sources up
to a separate 512 MiB hard ceiling, under the same time/memory/output limits.
Line counting scans from the source beginning to preserve exact line numbers.
Limits and incomplete ranges never imply the rest of a source was understood.

Text preserves original UTF-8, newlines and byte/line locators. Invalid encodings
are reported without lossy conversion. Raw private-key headers are excluded even
when a range skips the header. Bounded whole sources receive a full SHA-256;
larger sources have no digest rather than a misleading prefix hash. Changes to
source metadata during work invalidate the result. The broker remains
responsible for pathname/mount/grant checks that an open descriptor cannot prove.

PDF processing uses compile-pinned Poppler `pdfinfo` and `pdftotext` with fixed
arguments and a sealed, read-only memory snapshot at `/proc/self/fd/0`. Seals
prevent reopening it with write access. Original descriptors are close-on-exec,
and the worker disables parent ptrace/proc access before parsing. Snapshot
pages count toward the service memory cap. Parsers do
not receive the original path or broker sockets. Returned page numbers identify
physical pages, not printed page labels. Layout is best effort and graphics are
not interpreted. Automatic parsing stops at 200 pages; explicit bounded page
ranges can select later pages. Encrypted files require interactive opening;
this worker accepts no password. Scanned/image-only PDFs return metadata, not an
OCR claim. Images, archives, executables, databases and unsupported binary input
are metadata only. No archives, macros, formulas or external links are executed.

Retained text is limited to 2 MiB, metadata-tool output to 64 KiB per pipe, and
the complete JSON response to 16 MiB including escaping. The service has a
512 MiB aggregate memory cap, no swap, eight tasks, a ten-second CPU rlimit and
a ten-second whole-tree deadline. Its 90% aggregate CPU quota leaves deadline
scheduling headroom below the ten-second CPU budget. Deadline termination kills
the whole cgroup; a parser crash or resource failure cannot run inside its
caller. The broker surfaces transport/resource failure as a stable error instead of
an empty successful document.

`Files1.ExtractScoped` accepts a live content handle, typed format/range and a
response budget of 1–131072 bytes. Its one-use ticket retains the original
read-only descriptor. After parsing, the broker rechecks owner, grant expiry,
root/mount and source identity before publishing provenance and exact locators.
The grant lock is released during parsing, and root revocation has separate
admission capacity. Unicode-safe response clipping updates line/byte locators
and marks incompleteness. This method does not persist or index extracted text
and is not yet advertised to the model registry.

The optional `extract-native` integration provider installs the actual package
in a disposable, identity-verified NixOS test node. It exercises Unicode,
locators, input/output/page limits, large-source ranges, malformed/scanned/
encrypted PDFs and readonly descriptor checks. Fixed resource probes use the
production service profile with a different test executable to measure child
process OOM and CPU deadline behavior. This distinguishes parser corpus evidence
from resource-profile evidence. The disposable node disables the test driver's
panic-on-cgroup-OOM behavior; production limits and host settings are unaffected.

Run with `devctl test --suite integration --provider extract-native --detach`,
then inspect that job and retrieve terminal evidence with `devctl artifacts pull
--job UUID`. This optional test is outside default fast checks. It does not
qualify a production index, inference-aware scheduling or final OS release.
