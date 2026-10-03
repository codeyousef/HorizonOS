use aios_guard::Plan;
use std::io::{self, Read};
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--native-preflight"] {
        match aios_guard::native::NativeIntake::capture() {
            Ok(intake) => println!(
                "{}",
                serde_json::json!({"schema_version":1,"evidence":intake.evidence(),
                "native_artifact_intake_verified":true,"activation_performed":false,"authorization_verified":false,"runtime_adapter_available":false})
            ),
            Err(error) => {
                println!(
                    "{}",
                    serde_json::json!({"schema_version":1,"error":"GUARD_NATIVE_INTAKE_FAILED","reason":format!("{error:?}")})
                );
                std::process::exit(if error == aios_guard::Error::Authority {
                    5
                } else {
                    8
                });
            }
        }
        return;
    }
    if args == ["--check-plan"] {
        let mut bytes = vec![];
        let result = io::stdin()
            .take(65537)
            .read_to_end(&mut bytes)
            .ok()
            .and_then(|_| Plan::from_json(&bytes).ok());
        if let Some(plan) = result {
            if let Ok(digest) = plan.digest() {
                println!(
                    "{}",
                    serde_json::json!({"schema_version":1,"plan_digest":digest,"reboot_required":plan.needs_reboot(),"activation_performed":false,"runtime_adapter_available":false})
                );
                return;
            }
        }
        println!(
            "{}",
            serde_json::json!({"schema_version":1,"error":"INVALID_GUARD_PLAN"})
        );
        std::process::exit(2);
    }
    // Pure validation is available. No unqualified privileged activation path
    // or model-invocable command fallback is exposed by this initial package.
    println!(
        "{}",
        serde_json::json!({"schema_version":1,"error":"GUARD_RUNTIME_ADAPTER_UNAVAILABLE"})
    );
    std::process::exit(9);
}
