//! CLI -> real peer-authenticated Unix API -> native systemd.
use std::{fs, os::unix::{fs::PermissionsExt,net::UnixListener}, process::Command, sync::{Arc,Mutex},thread};
use aios_session::{State, serve_connection};
use serde_json::Value;
#[test]
fn cli_inspects_sshd_without_inference_or_privilege() {
    let directory=std::path::PathBuf::from("/tmp").join(format!("aios-cli-ipc-{}",std::process::id()));
    fs::create_dir(&directory).unwrap();fs::set_permissions(&directory,fs::Permissions::from_mode(0o700)).unwrap();
    let socket=directory.join("session.sock");let listener=UnixListener::bind(&socket).unwrap();
    let state=Arc::new(Mutex::new(State::default()));
    let server=thread::spawn(move|| {let(stream,_)=listener.accept().unwrap();serve_connection(stream,state).unwrap();});
    let output=Command::new(env!("CARGO_BIN_EXE_aiosctl")).args(["inspect","service","sshd.service","--json","--socket",socket.to_str().unwrap()]).output().unwrap();
    server.join().unwrap();fs::remove_dir_all(directory).unwrap();
    assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
    let value:Value=serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"],"ok");assert_eq!(value["data"]["active_state"],"active");
    println!("AIOS_CLI_SERVICE={value}");
}
