# Version identifiers

`VERSION` identifies the prerelease product. Public releases follow semantic
versioning. Protocol/schema major versions are independent compatibility contracts;
breaking supported interfaces require an explicit version change and migration plan.

NixOS generation numbers, graph revisions, product releases, model/runtime/template
hashes and protocol versions are separate identifiers. Never substitute one for
another. `system.stateVersion` remains the installation baseline during upgrades.

External selections become release inputs only after exact revisions and artifact
hashes are discovered, locked and verified. Unresolved selections cannot qualify
release artifacts. Rollback must respect supported policy/data-schema compatibility
and application-consistent data migration barriers.
