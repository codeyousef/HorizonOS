//! Pure installed-catalog checker, never a mutation or approval interface.
use aios_state::Catalog;
use std::io::{self, Read};
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let Some(installed) = option_env!("AIOS_STATE_CATALOG_JSON") else {
        println!(
            "{}",
            serde_json::json!({"schema_version":1,"error":"INSTALLED_CATALOG_UNAVAILABLE"})
        );
        std::process::exit(9);
    };
    let result = (|| {
        let catalog = Catalog::from_installed(installed.as_bytes())
            .map_err(|_| "INSTALLED_CATALOG_INVALID")?;
        if args == ["--catalog"] {
            return Ok(serde_json::from_str::<serde_json::Value>(installed).unwrap());
        }
        if args == ["--defaults"] {
            return Ok(serde_json::to_value(catalog.defaults()).unwrap());
        }
        if args != ["--check-manifest"] && args != ["--preview"] {
            return Err("CHECK_INTERFACE_ONLY");
        }
        let mut bytes = vec![];
        io::stdin()
            .take((aios_state::MAX_MANIFEST_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "INVALID_MANIFEST")?;
        if args == ["--preview"] {
            let preview = catalog
                .preview_request(&bytes)
                .map_err(|_| "INVALID_PREVIEW_REQUEST")?;
            return Ok(
                serde_json::json!({"schema_version":1,"source":"offline_manifest",
                "installed_baseline_verified":false,"activation_performed":false,"authorization_granted":false,"preview":preview}),
            );
        }
        let compiled = catalog.compile(&bytes).map_err(|_| "INVALID_MANIFEST")?;
        Ok(
            serde_json::json!({"schema_version":1,"managed_sha256":compiled.digest,"managed":compiled.state,
            "activation_performed":false,"authorization_granted":false}),
        )
    })();
    match result {
        Ok(value) => println!("{value}"),
        Err(error) => {
            println!("{}", serde_json::json!({"schema_version":1,"error":error}));
            std::process::exit(2);
        }
    }
}
