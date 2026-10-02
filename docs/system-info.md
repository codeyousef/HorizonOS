# Deterministic system information

`aiosctl system info --json` observes the local NixOS installation without a
model, session attachment or privileged execution. It returns the versioned
provider envelope and actual OS version, running NixOS closure, generation,
boot ID, architecture and upstream virtualization detection. The generation
is reported only when the system profile resolves to the observed running
closure; unresolved or divergent state is partial rather than invented.

The first typed action is `system.info`, with exactly empty arguments. The
request parser rejects unknown fields, duplicate envelope fields, oversized
requests, unknown action IDs and client-supplied authority. Normative schemas
are in `schemas/api` and `schemas/actions`. Other actions and schema-to-Rust
generation are still being implemented. The current command invokes the
read-only provider locally; the authenticated public D-Bus/session request
lifecycle and persistent scoped evidence handles are separate requirements.

Build the `aios-cli` flake package inside the verified development guest. The
host command `python3 tools/devctl.py test --suite integration --provider
system-info --json` runs the registered real KVM provider smoke. It compares
the reported boot/closure with actual guest files and tests refusal of shell
and authority flags. It does not imply all system/provider integration passes.
