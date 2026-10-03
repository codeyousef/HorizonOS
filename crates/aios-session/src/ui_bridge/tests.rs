//! Kernel ancillary transport fixtures, not graphical consent evidence.
use super::*;
#[test]
fn original_kernel_connection_is_transferred_with_cloexec_and_native_peer_credentials(){
    let (bridge,receiver)=UnixStream::pair().unwrap();let (origin,_client)=UnixStream::pair().unwrap();
    send_proof(&bridge,&origin).unwrap();let transferred=receive_proof(&receiver).unwrap();
    let before=crate::identity::authenticate(&origin).unwrap();let after=crate::identity::authenticate(&transferred).unwrap();assert_eq!(before,after);
    assert_ne!(origin.as_raw_fd(),transferred.as_raw_fd());
    let flags=unsafe{nix::libc::fcntl(transferred.as_raw_fd(),nix::libc::F_GETFD)};
    assert!(flags&nix::libc::FD_CLOEXEC!=0);
}
#[test]
fn missing_multiple_and_wrong_marker_descriptors_are_denied(){
    for count in [0,2,253]{
        let (bridge,receiver)=UnixStream::pair().unwrap();let (origin,_client)=UnixStream::pair().unwrap();
        let fds=vec![origin.as_raw_fd();count];
        let controls=if count==0{vec![]}else{vec![ControlMessage::ScmRights(&fds)]};
        sendmsg::<()>(bridge.as_raw_fd(),&[IoSlice::new(&[MARKER])],&controls,MsgFlags::MSG_NOSIGNAL,None).unwrap();
        assert!(matches!(receive_proof(&receiver),Err(ErrorCode::PermissionDenied)));
    }
    let (bridge,receiver)=UnixStream::pair().unwrap();let (origin,_client)=UnixStream::pair().unwrap();
    sendmsg::<()>(bridge.as_raw_fd(),&[IoSlice::new(&[0])],&[ControlMessage::ScmRights(&[origin.as_raw_fd()])],MsgFlags::MSG_NOSIGNAL,None).unwrap();
    assert!(matches!(receive_proof(&receiver),Err(ErrorCode::PermissionDenied)));
}
#[test]
fn private_bridge_rejects_claimed_approval_identity_and_duplicate_fields(){
    for raw in [r#"{"kind":"discover","session_id":"1","uid":1001}"#,
        r#"{"kind":"start_read","window_handle":"x","goal":"read","mode":"ask","approved":true}"#,
        r#"{"kind":"start_read","window_handle":"x","goal":"a","goal":"b","mode":"ask"}"#,
        r#"{"kind":"cancel","task_id":"x","decision":"allow"}"#,
        r#"{"kind":"approve","task_id":"x"}"#]{assert!(matches!(parse(raw),Err(ErrorCode::InvalidArgument)));}
}
