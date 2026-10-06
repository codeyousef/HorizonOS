# Optional developer cache

This separate flake pins Kache v0.28.1 and its compiler/package inputs. It does
not change the main OS lock, install a system service or configure a compiler
wrapper. Build and trial it only in an enrolled development guest.

For a trial, use an explicit `KACHE_CONFIG` based on `trial.example.toml`, empty
`KACHE_HOST_CONFIG`, `KACHE_LOCAL_ONLY=1`, and explicit private cache, runtime
and socket paths. Allocate new cold/warm Cargo targets on the same filesystem.
Keep generated guest storage in the managed disks under `/mnt/Storage`.
Do not run `kache init` or use existing targets. Reserve recovery space before
downloads/builds and measure identical outputs, hits, elapsed time and allocated
shared blocks before enabling the wrapper beyond a trial.

The configured blob budget is not a filesystem quota. Shared target outputs
can retain blocks beyond that budget. Automatic target cleanup, incremental
cleanup, index compaction and GC are disabled in the trial configuration.
