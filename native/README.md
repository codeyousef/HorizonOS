# Native interfaces

`llama-bridge` will expose a narrow C ABI over the locked CPU runtime. `kde`
contains the Qt6/KF6 desktop integrations; `app-adapters` contains compiled,
application-specific adapters. They use versioned authenticated AIOS APIs.
No generic privileged D-Bus or shell bridge is permitted. Executable targets
are added alongside their implementations and guest verification.
