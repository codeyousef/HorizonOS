# Horizon OS model artifacts

The normal candidate is the official Qwen3.5-2B checkpoint converted locally
to Q4_K_M. `source-lock.json` pins original file sizes and SHA-256 hashes, the
upstream revision, license, runtime source and context allocation. `lock.json`
pins the resulting GGUF, converter, quantizer and embedded chat template.
Unresolved development selections cannot be loaded or used in a release.

Artifact fetch and conversion are separate supervised development operations.
The installed native inference path has no Python/PyTorch dependency or download
API. Model weights are independent of small code rebuilds. Production loading
accepts root-owned read-only Nix store artifacts; the qualification executable
also accepts the current guest user's read-only converted artifact.

The project-owned native ABI builds the locked llama.cpp CPU variants with
runtime dispatch. GPU/RPC/BLAS backends and native-host compiler optimization
are disabled. Rust verifies the artifact through a retained file descriptor
before native loading, verifies the embedded template hash, owns bounded prompt
and token buffers, and keeps cancellation independent of context ownership.
Every request has a fresh context. Extended thinking is disabled through the
upstream Jinja template mechanism; hidden text is never parsed as an action.
Backend path overrides are rejected before dynamic library loading. Writable,
linked qualification files, special files, symlinks and hash mismatches cannot
reach the native model loader.

Compatibility probes establish loading, template use, constrained output and
cancellation. They do not establish held-out answer quality, factual citation
verification, resource benchmarks or a completed model daemon. Those release
gates must be qualified separately with the actual pinned model.
