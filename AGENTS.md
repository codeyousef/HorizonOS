# Minnerite engineering workflow

Read the linked Linear issue and latest comments before implementation. Linear
holds private planning, blockers, requirement status and session handoffs. Keep
tracked documentation about the product and its engineering contracts.

The canonical specification is the Minnerite PRD v1.0 in Linear. Preserve its
AIOS interface names. The fixed implementation is NixOS, Rust, Qt/KDE and local
CPU inference. Read the relevant PRD sections before changing a subsystem.

Codex, Git and SSH private keys stay on the host. Build and runtime verification
happen in verified NixOS guests. Never administer the host as the guest, copy
host credentials into it, or mount host private directories. Host doctor and
Python host-tool unit tests are allowed on the host. Do not install prerequisites
or change host virtualization/network/boot permissions automatically.

Before every guest operation, verify pinned SSH trust, NixOS identity, installation
UUID, DMI UUID and guest role. Mutations also verify disk and management identity.
Target mismatches stop execution. Missing enrollment is not permission to skip
checks. See PRD sections 5, 7, 17 and 18.

The model proposes typed actions. Deterministic code owns authorization, execution,
verification and recovery. Never add unrestricted shell, arbitrary Nix, generic
privileged D-Bus or policy bypass tools. Observations/documents are not user intent.

Link real verification evidence in Linear. Label fixtures and unverified guest
behavior accurately. End sessions with changes, blocker, next exact step, branch
and exact validation commands. Do not create tracked progress/handoff ledgers.
