# Horizon OS

Horizon OS implements the AI-native NixOS specification under the internal `aios-*`
interface names. A deterministic control plane authorizes, executes, verifies and
recovers typed actions; local CPU inference interprets requests and evidence.

[Project and delivery issues](https://linear.app/felidai-studio/project/horizon-os-978454c1cb4a)
and [full PRD v1.0](https://linear.app/felidai-studio/document/horizon-os-full-prd-v10-aios-nixos-11afca2bea89)
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
Guest SSH/build/deployment operations report `UNSUPPORTED_CAPABILITY` until
trusted enrollment and their implementations exist.

## Engineering boundaries

Codex and credentials stay on the host. Source snapshots, builds and OS tests run
in guests. Images, keys and sanitized reports belong under ignored `.local/`.
Guest identity mismatch is a hard stop. See [architecture](docs/architecture.md),
[threat model](docs/threat-model.md) and [versioning](docs/versioning.md).

The Rust workspace begins with protocol framing and test fixtures. The reusable
NixOS module refuses AI enablement until the services exist. The initial flake
exposes host tooling and its check; OS/model/desktop/install/recovery outputs will
be added with their implementations. These sources are not a complete OS or
evidence of guest build, model quality, rollback or GUI reliability.

Nixpkgs and the stable Rust channel are initial selections. `flake.lock` and a
pinned Rust release must be generated and validated through the M0 upstream and
guest workflow. No external lock hashes are fabricated here. The Rust crates
currently use only the standard library.
