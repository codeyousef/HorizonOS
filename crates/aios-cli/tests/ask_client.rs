use std::process::Command;

#[test]
fn standalone_client_rejects_shell_and_authority_modes_before_transport() {
    for arguments in [
        &["id", "--mode", "shell", "--json"][..],
        &["id", "--shell", "--json"][..],
        &["id", "--mode", "read-only", "--json", "--approved"][..],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_ask"))
            .args(arguments)
            .output()
            .expect("standalone ask client starts");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "Usage: ask TEXT [--mode read-only] --json\n"
        );
    }
}
