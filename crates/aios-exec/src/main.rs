//! Product transport/authorization and live adapters are not yet connected.
fn main() {
    let root = unsafe { libc::getuid() } == 0 && unsafe { libc::geteuid() } == 0;
    println!(
        "{}",
        serde_json::json!({"schema_version":1,"error":if root{"BROKER_RUNTIME_ADAPTER_UNAVAILABLE"}else{"BROKER_AUTHORITY_REQUIRED"}})
    );
    std::process::exit(if root { 9 } else { 5 });
}
