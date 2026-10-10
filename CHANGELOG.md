# Changelog

## Unreleased

- Rename the legacy product and user-visible branding to Minnerite
  while retaining the PRD-mandated `aios-*` executable, service, protocol,
  schema, and configuration interfaces.
- Define the complete typed `services.aios` module option surface with safe
  defaults, cross-option assertions, generated documentation coverage, and
  explicit model-independent desktop-image composition.
- Add disposable-image service lifecycle qualification covering fixed system and
  user unit restarts, effective systemd isolation, private socket/storage access
  plans, model-disabled operation, logout cleanup, and independent SSH, network,
  and display availability.
- Add an isolated `aios-observer` account and hardened `aios-observer.service`
  that maintains a private, process-bound status record for fixed systemd and
  udev subscriptions while exposing neither event payloads nor mutation/network
  authority.
- Add a root-authenticated, network-isolated `aios-build` worker that reopens
  sealed Executor1 candidates, rejects unapproved templates, substituters,
  baselines and resource bounds, builds only the fixed reviewed system target,
  inventories closure deltas, and retains verified prior/candidate indirect GC
  roots without activation or boot-selection authority.
- Connect the root-only native guard adapter to a durable Executor1 handoff and
  independently supervised template service. It revalidates enrolled target and
  immutable closure artifacts, retains exact recovery roots, probes fixed
  system/user/API/action health, accepts transaction-bound heartbeats only
  through the authenticated SSH management session, commits exact pointers, and
  reconciles rollback or post-commit disarming after restart.
- Add the production-separated VM developer deployment guard. The installed
  root helper builds only a registered immutable `aios-dev` snapshot, test-applies
  it under the retained native guard and rolls back, then commits only that
  tested exact closure through a distinct nonce-bound transaction. Production
  evaluation excludes the helper, unit and sudo route.
- Verify factual answers against request-scoped evidence before rendering.
  Retrieval uses fixed provider, installed-documentation, authorized-file, then
  separately authorized external ordering; stale versions, cross-scope IDs,
  forged locators, unsupported external freshness, and mismatched structured
  numeric claims fail closed. Rendered locators never carry execution authority.
- Add consented per-user file roots and opaque scoped handles. Root proposals are
  limited to existing XDG Documents, Downloads and Desktop directories on the
  home mount; descriptor-relative `openat2` resolution denies symlink, traversal
  and mount escapes. UID/session, root, file metadata and expiry are revalidated
  for every metadata/content/mutation capability. The user broker retains the
  caller's user/home namespace so those descriptor capabilities can reach the
  selected roots; it retains no capabilities or network access. Revocation
  blocks handles before purging cached chunks, previews and snippets, and fixed
  secret paths remain excluded after enrollment.
- Bind the independent guard's target identity and prior running, profile, and
  boot closures to the immutable prepared baseline. Activation now closes an
  intervening Nix profile generation or target change as a terminal rejection
  instead of adopting it as the rollback baseline or leaving authorization live.
- By default, retain the newest three distinct verified system profiles as root-owned GC
  roots after commit or rollback, independently of active transaction roots;
  malformed retention state fails closed. System profile history remains
  explicitly unmanaged until transaction provenance is independently joined.
- Protect the active guard with NixOS non-stop/non-restart/non-removal switch
  metadata instead of `RefuseManualStop`; the latter rejected the entire
  candidate and recovery switch transaction rather than preserving it.
- Let the root-only guard's fixed immutable activation commands use the
  filesystem, kernel and user-runtime activation paths that NixOS requires;
  broad `ProtectSystem`, `ProtectKernelTunables`, and `ProtectHome` mounts made
  candidate and recovery activation fail read-only while command, identity and
  artifact gates remain.
- Resolve desktop applications delegated to the per-user systemd manager through
  logind's root-owned `User.Display` pointer for the same UID, while refusing to
  substitute a graphical session for a concrete SSH or TTY caller.
- Permit broker-owned foreground-terminal confirmations from a Konsole process
  in the caller's active local Wayland/X11 logind session; terminal ownership,
  foreground process-group, UID, boot and session bindings remain mandatory.
- Add authenticated `aiosctl plan` and transaction inspection, authorization,
  apply, cancel and rollback-plan commands for bounded package intents. Cancel
  exposes the broker's owner-bound pre-effect cancellation. Housekeeping closes
  an expired prepared plan against its preparation deadline and an expired
  awaiting-approval plan against its separate frozen final deadline, without
  cancelling a verified build merely because resource consent expired. Prepared
  plans persist only for the same UID, boot, system bus and live logind session;
  headless denials preserve the concrete plan ID instead of claiming consent.
- Add broker-owned final-plan TTY confirmation with foreground process/session
  checks, control-safe immutable impact/recovery rendering, an exact plan-bound
  phrase, and mandatory separate fresh native polkit authentication.
- Keep final native approval and guarded activation on the same authenticated
  `aiosctl transaction apply` process and system-bus connection, so exact
  process-bound receipts are usable without weakening their caller binding.
- Permit the typed guard status and heartbeat D-Bus members through the
  Executor1 bus policy; runtime management-session, owner, transaction, nonce,
  target, and digest checks remain mandatory.
- Accept the ledger's immutable build closure at guard handoff while requiring
  the prepared semantic preview to remain pre-build; the guard no longer
  compares that intentionally empty preview field to the realized closure.
- Resolve immutable Nix store links before hashing managed manifests, while
  rejecting targets outside `/nix/store` and retaining no-follow, ownership,
  mode, size, and race checks on the resolved file. Retained Nix invocation
  accepts the immutable `nix-env` alias resolving to the package's `nix`
  executable instead of rejecting that standard Nix layout. Guard service
  identity likewise resolves NixOS's `/etc/systemd/system` fragment link before
  enforcing its component-relative immutable store suffix, root ownership, and
  mode. Guard failures identify their fixed startup stage without logging
  mutable paths or request content.
- Add a production-excluded disposable administrator with a fixed test-only
  credential for real native polkit challenge qualification.
- Disable polkit 127's broken socket-activated PAM helper so native
  authorization uses the standard setuid helper until the pinned upstream is
  fixed.
- Keep broker cancellation responsive during final-plan TTY and polkit waits;
  cancellation withdraws the native challenge and cannot race into a receipt.
- Connect preliminary foreground-TTY resource confirmation to the installed
  isolated build worker, independently authenticate its process/socket/output
  and retained roots, and freeze only a rechecked exact final plan.
- Terminalize a failed candidate build only after the authenticated single-flight
  worker returns a request-bound completion response, releasing the active-plan
  slot without treating a timeout or disconnected worker as stopped.
- Add model-independent `aiosctl automation list` output that truthfully reports
  no persistent definitions while scheduling remains unavailable.
- Keep disposable desktop QMP and serial endpoints within the portable Unix
  socket bound when qualification is launched from a nested managed workspace.
- Ship standalone `ask` and `aiosctl ask --mode read-only` clients with human
  answer/citation output, optional typed JSON, and no shell-generation path.
- Add human-readable `aiosctl inspect service` output while retaining complete
  typed observations behind `--json`.
- Add deterministic `aiosctl model status` and `model unload` controls through
  the caller-authenticated session API without invoking inference.
- Add model-independent package catalog queries and aggregate graph health to
  `aiosctl` through authenticated fixed D-Bus methods.
- Expose model-independent authenticated privacy-scope and volatile-history
  metadata commands without disclosing prompt or answer text.
- Recover verified installer downloads after interrupted partial transfers instead
  of leaving a cache entry that blocks every subsequent clean installation.
- Isolate each QEMU process in a collected per-VM systemd user service with
  bounded resident memory and no swap, so guest pressure cannot kill the host
  controller session.
- Reject every repository-defined disposable acceptance profile and fixture unit
  when evaluating the production NixOS module.
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
  separate native owners without cross-user profile leakage, and report
  ephemeral runtime and unmanaged non-profile inventory as distinct, explicitly
  incomplete domains rather than conflating or inferring either.
- Add caller-scoped `system.hardware` and `storage.status` observations using
  bounded CPU, libudev/sysfs, mountinfo and filesystem-stat reads; serial values
  stay redacted, missing hardware stays unsupported, and no storage effect exists.
- Add authenticated `network.status` and `bluetooth.status` reads over fixed
  NetworkManager, BlueZ and rfkill sources with caller-scoped handles, distinct
  network domains and explicit missing-radio results; their typed R3/R2 writes
  remain unavailable pending native approval, verification and recovery.
- Admit only Unix and netlink sockets to the unprivileged graph owner so its
  bounded libudev monitor receives kernel events without gaining IP networking;
  qualify removable-media and full-volume behavior in disposable images.
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
