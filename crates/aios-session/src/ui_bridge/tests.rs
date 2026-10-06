//! Kernel ancillary transport fixtures, not graphical consent evidence.
use super::*;
#[test]
fn original_kernel_connection_is_transferred_with_cloexec_and_native_peer_credentials(){
    let (bridge,receiver)=UnixStream::pair().unwrap();let (origin,_client)=UnixStream::pair().unwrap();
    send_proof(&bridge,&origin).unwrap();let TransferredProof::Unix(transferred)=receive_proof(&receiver).unwrap() else {panic!("wrong transport");};
    let before=crate::identity::authenticate(&origin).unwrap();let after=crate::identity::authenticate(&transferred).unwrap();assert_eq!(before,after);
    assert_ne!(origin.as_raw_fd(),transferred.as_raw_fd());
    let flags=unsafe{nix::libc::fcntl(transferred.as_raw_fd(),nix::libc::F_GETFD)};
    assert!(flags&nix::libc::FD_CLOEXEC!=0);
}
#[test]
fn bus_handoff_accepts_no_descriptors_and_rejects_serialized_identity(){
    let (bridge,receiver)=UnixStream::pair().unwrap();
    sendmsg::<()>(bridge.as_raw_fd(),&[IoSlice::new(&[BUS_MARKER])],&[],MsgFlags::MSG_NOSIGNAL,None).unwrap();
    assert!(matches!(receive_proof(&receiver),Ok(TransferredProof::Bus)));
    for count in [1,2,253]{
        let (bridge,receiver)=UnixStream::pair().unwrap();let (origin,_client)=UnixStream::pair().unwrap();
        sendmsg::<()>(bridge.as_raw_fd(),&[IoSlice::new(&[BUS_MARKER])],&[ControlMessage::ScmRights(&vec![origin.as_raw_fd();count])],MsgFlags::MSG_NOSIGNAL,None).unwrap();
        assert!(matches!(receive_proof(&receiver),Err(ErrorCode::PermissionDenied)));
    }
    for raw in [r#"{"schema_version":1,"sender":":1.1","bus_id":"x","uid":1001}"#,
        r#"{"schema_version":1,"sender":":1.1","sender":":1.2","bus_id":"x"}"#,
        r#"{"schema_version":1,"sender":":1.1","bus_id":"x","approved":true}"#]{
        assert!(serde_json::from_str::<BusReference>(raw).is_err());
    }
    for (sender,id) in [("org.aios.Session1","00000000000000000000000000000000"),(":1.1","short"),(":1.1","gggggggggggggggggggggggggggggggg")]{
        assert_eq!(crate::user_bus::validate_reference(sender,id),Err(ErrorCode::InvalidArgument));
    }
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
#[test]
fn public_task_channel_has_no_approval_fields_and_transfers_only_one_native_socket(){
    for raw in [r#"{"kind":"start_task_read","task_id":"x","window_handle":"w","goal":"read","mode":"ask","approved":true}"#,
        r#"{"kind":"start_task_read","task_id":"a","task_id":"b","window_handle":"w","goal":"read","mode":"ask"}"#,
        r#"{"kind":"start_task_read","task_id":"x","window_handle":"w","goal":"read","mode":"ask","uid":0}"#]{
        assert!(matches!(parse(raw),Err(ErrorCode::InvalidArgument)));
    }
    let (bridge,receiver)=UnixStream::pair().unwrap();let (cancel,endpoint)=Cancellation::pair().unwrap();
    send_proof(&bridge,&endpoint).unwrap();
    let TransferredProof::Unix(mut transferred)=receive_proof(&receiver).unwrap() else{panic!("not a socket");};
    assert_eq!(crate::identity::authenticate(&endpoint).unwrap(),crate::identity::authenticate(&transferred).unwrap());
    drop(endpoint);transferred.set_read_timeout(Some(Duration::from_millis(100))).unwrap();
    cancel.cancel();assert_eq!(transferred.read(&mut [0]).unwrap(),0);
    // Dropping a pending task has the same revocation behavior.
    let (cancel,mut receiver)=Cancellation::pair().unwrap();drop(cancel);assert_eq!(receiver.read(&mut [0]).unwrap(),0);
}

#[test]
fn paging_rejects_serialized_authority_paths_and_duplicate_generations(){
    for raw in [
        r#"{"kind":"page_snapshot","task_id":"t","snapshot_id":"s","container_handle":"h","approved":true}"#,
        r#"{"kind":"page_snapshot","task_id":"t","snapshot_id":"s","snapshot_id":"other","container_handle":"h"}"#,
        r#"{"kind":"page_snapshot","task_id":"t","snapshot_id":"s","container_handle":"h","object_path":"/org/a11y/atspi/accessible/root"}"#,
        r#"{"kind":"snapshot_containers","task_id":"t","snapshot_id":"s","grant":{}}"#,
    ]{assert!(matches!(parse(raw),Err(ErrorCode::InvalidArgument)));}
}

#[test]
fn selectors_reject_duplicate_nested_fields_and_forged_authority(){
    let id="11111111-1111-4111-8111-111111111111";
    let raw=format!("{{\"kind\":\"find_nodes\",\"task_id\":\"{id}\",\"snapshot_id\":\"{id}\",\"selector\":{{\"name\":\"Document\"}}}}");
    assert!(parse(&raw).is_ok());
    for altered in [raw.replace("Document\"","Document\",\"name\":\"other\""),raw.replace("\"kind\":","\"approved\":true,\"kind\":")] {assert!(matches!(parse(&altered),Err(ErrorCode::InvalidArgument)));}
}
