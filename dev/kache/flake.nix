{
  description = "Optional isolated Horizon OS developer Rust cache";

  # Keep the tool's compiler/package inputs separate from the OS package base.
  # v0.28.1, resolved to its immutable commit; flake.lock pins the full graph.
  inputs.kache.url = "github:kunobi-ninja/kache/3cd7e831171de36423002169f3d7053675635a06";

  outputs = { self, kache }: {
    packages.x86_64-linux.default = kache.packages.x86_64-linux.kache;
  };
}
