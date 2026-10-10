# Minnerite

Minnerite implements the AI-native NixOS specification under the internal `aios-*`
interface names. A deterministic control plane authorizes, executes, verifies and
recovers typed actions; local CPU inference interprets requests and evidence.

[Project and delivery issues](https://linear.app/felidai-studio/project/minnerite-978454c1cb4a)
and [full PRD v1.0](https://linear.app/felidai-studio/document/minnerite-full-prd-v10-11afca2bea89)
are the canonical task and specification context.

## Host entry points

Python 3 standard-library tooling runs on the Linux host:

```sh
python3 tools/devctl.py doctor --host --json
python3 -m unittest discover -s tests/unit -v
```

The doctor reads resources, prerequisite versions, configured paths and local
connectivity. It does not install packages, change groups, enroll SSH keys or
write VM configuration. Missing prerequisites are reported with exit status 3.

Copy `dev/vm.example.json` to ignored `.local/vm.json` only when configuring a
target. `vm create` prepares a reviewable plan; its UUID authorizes preparation
of a fresh virtual disk. The protected QEMU bootstrap uses official installer
media and a read-only public seed. See [bootstrap deployment](docs/deployment.md).
Guest operations require trusted enrollment. Registered build/test jobs verify
the target and source snapshot before executing in the NixOS guest.

## Engineering boundaries

Codex and credentials stay on the host. Source snapshots, builds and OS tests run
in guests. Images, keys and sanitized reports belong under ignored `.local/`.
Guest identity mismatch is a hard stop. See [architecture](docs/architecture.md),
[threat model](docs/threat-model.md) and [versioning](docs/versioning.md).

The flake packages host tooling, the authenticated user session service, the
read-only CLI and a native CPU model qualification executable. The reusable
NixOS module refuses full AI enablement until the service contract is complete.
See [system observation](docs/system-info.md), [service inspection](docs/service-inspection.md)
and [model artifacts](models/README.md) for their engineering contracts.

`flake.lock`, `Cargo.lock` and the model manifests pin the dependencies and model
provenance. Build and runtime checks belong in the verified guest. A model
compatibility probe establishes native loading and constrained generation;
held-out quality, resource targets, desktop behavior and recovery require their
own acceptance evidence.
