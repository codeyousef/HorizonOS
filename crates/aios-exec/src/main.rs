//! Fixed native startup preflight; transport/authorization remain unavailable.
fn main() {
    let root = unsafe { libc::getuid() } == 0 && unsafe { libc::geteuid() } == 0;
    if root {
        let preflight = (|| -> aios_exec::Result<()> {
            let target = aios_exec::native::VerifiedTarget::enroll()?;
            let _template = aios_exec::candidate::InstalledTemplate::from_installed()?;
            target.recheck()
        })();
        if let Err(reason) = preflight {
            println!(
                "{}",
                serde_json::json!({"schema_version":1,"error":"BROKER_NATIVE_PREFLIGHT_FAILED","reason":format!("{reason:?}")})
            );
            std::process::exit(9);
        }
    }
    println!(
        "{}",
        serde_json::json!({"schema_version":1,"error":if root{"BROKER_RUNTIME_ADAPTER_UNAVAILABLE"}else{"BROKER_AUTHORITY_REQUIRED"}})
    );
    std::process::exit(if root { 9 } else { 5 });
}
