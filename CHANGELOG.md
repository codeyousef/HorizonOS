# Changelog

## Unreleased

- Host discovery and validated development-target configuration.
- Keep native UI Stop and Forget available after a stale selector refusal while
  revoking the failed read without retry.
- Add privacy-safe fixed-stage diagnostics for native process-termination setup
  failures without logging task or target data.
- Add opaque R1 task grants bound to authenticated subject, exact scope, policy,
  plan, live resources, broker incarnation, volatile nonce and expiry.
- Expose immutable reviewed package-catalog provenance separately from managed
  selection, and verify declared runtime artifacts against the running Nix closure.
- Export a revision-bound typed catalog for the seven managed configuration
  options with explicit access and completeness semantics.
- Enumerate bounded system and per-user profile generation history using
  separate native owners without cross-user profile leakage, while keeping
  ephemeral inventory and unattributed management provenance explicitly unknown.
- Add authenticated read-only audio inventory/default and power status D-Bus
  providers with stable handles, bounded native queries, truthful partial fields
  and write methods that remain approval-gated.
- Initial Rust protocol framing and NixOS engineering scaffold.
