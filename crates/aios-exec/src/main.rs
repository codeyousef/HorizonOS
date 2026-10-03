//! Fixed native preflight and authenticated system-bus daemon entry points.
fn main() {
    let root = unsafe { libc::getuid() } == 0 && unsafe { libc::geteuid() } == 0;
    if !root {
        println!(
            "{}",
            serde_json::json!({"schema_version":1,"error":"BROKER_AUTHORITY_REQUIRED"})
        );
        std::process::exit(5);
    }
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--native-preflight"] {
        let mut stage = "target-enrollment";
        let preflight = (|| -> aios_exec::Result<()> {
            let target = aios_exec::native::VerifiedTarget::enroll()?;
            stage = "installed-template";
            let _template = aios_exec::candidate::InstalledTemplate::from_installed()?;
            stage = "native-approval-authority";
            let _approval_engine = aios_exec::approval::Authorizer::open()?;
            stage = "target-recheck";
            target.recheck()
        })();
        if let Err(reason) = preflight {
            println!(
                "{}",
                serde_json::json!({"schema_version":1,"error":"BROKER_NATIVE_PREFLIGHT_FAILED","stage":stage,"reason":format!("{reason:?}")})
            );
            std::process::exit(9);
        }
        println!(
            "{}",
            serde_json::json!({"schema_version":1,"native_preflight_verified":true,"request_transport_available":true,"authorization_available":false,"activation_available":false})
        );
    } else if args.is_empty() || args == ["--serve"] {
        if let Err(reason) =
            aios_exec::bus::restrict_execution().and_then(|_| aios_exec::bus::serve())
        {
            println!(
                "{}",
                serde_json::json!({"schema_version":1,"error":"BROKER_RUNTIME_FAILED","reason":format!("{reason:?}")})
            );
            std::process::exit(9);
        }
    } else {
        println!(
            "{}",
            serde_json::json!({"schema_version":1,"error":"INVALID_ARGUMENT"})
        );
        std::process::exit(2);
    }
}
