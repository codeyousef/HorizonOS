# Changelog

## Unreleased

- Host discovery and validated development-target configuration.
- Keep native UI Stop and Forget available after a stale selector refusal while
  revoking the failed read without retry.
- Add privacy-safe fixed-stage diagnostics for native process-termination setup
  failures without logging task or target data.
- Add opaque R1 task grants bound to authenticated subject, exact scope, policy,
  plan, live resources, broker incarnation, volatile nonce and expiry.
- Expose immutable reviewed package-catalog metadata as provenance nodes without
  misrepresenting catalog membership as installed/runtime availability.
- Initial Rust protocol framing and NixOS engineering scaffold.
