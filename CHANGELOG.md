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
- Keep executor catalog/transaction fixture catalogs aligned with the complete
  reviewed managed-option schema so policy qualification exercises real plans.
- Bind file, application and UI evidence locators to provider-issued concrete
  identity digests; trusted viewers receive no model/display URI.
- Enumerate bounded system and per-user profile generation history using
  separate native owners without cross-user profile leakage, while keeping
  ephemeral inventory and unattributed management provenance explicitly unknown.
- Add authenticated read-only audio inventory/default, power status and the
  three registered desktop-setting adapters with stable handles, pinned bounded
  native queries, truthful unsupported/partial fields and write methods that
  remain approval-gated.
- Add an authenticated R1 task-action route for exact audio default/mute and
  desktop light/dark changes with opaque grants, live target revalidation,
  provider readback and a typed prior-state recovery action; direct provider
  writes remain denied.
- Add bounded PowerDevil display-idle and UPower keyboard-backlight task
  actions with stale-state refusal, exact native readback and typed recovery.
- Admit the fixed argument shapes for every registered R0 native audio and
  power read through the direct-read grant instead of rejecting them at policy
  revalidation.
- Emit a schema-valid `PARTIAL_RESULT` error object with incomplete native
  observations instead of discarding truthful power data during validation.
- Parse WirePlumber's actual `[vol: … MUTED]` status form and wait boundedly
  for asynchronous default/mute readback before issuing a verified receipt.
- Add the R2 native-confirmed PowerDevil profile route with allowlisted choices,
  stale-state and caller/desktop checks, bounded readback and prior-profile
  recovery; direct and R1 execution remain denied.
- Project only the two registered KDE settings files into the sandboxed
  session broker after late Plasma initialization or atomic configuration
  replacement.
- Initial Rust protocol framing and NixOS engineering scaffold.
