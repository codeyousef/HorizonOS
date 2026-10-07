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
- Initial Rust protocol framing and NixOS engineering scaffold.
