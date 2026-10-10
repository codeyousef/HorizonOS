use serde_json::{json, Value};
use std::{env, path::PathBuf, time::Duration};
use zbus::blocking::Proxy;

const NAME:&str="org.aios.Session1";
const PATH:&str="/org/aios/Files1";
const INTERFACE:&str="org.aios.Files1";

fn code(error:zbus::Error,expected:&str){
    let zbus::Error::MethodError(name,_,_)=error else{panic!("unexpected error: {error}")};
    assert_eq!(name.as_str(),format!("org.aios.Error.{expected}"));
}
fn value(proxy:&Proxy<'_>,method:&str,request:Option<&str>)->Value{
    let text:String=match request{Some(request)=>proxy.call(method,&(request,)).unwrap(),None=>proxy.call(method,&()).unwrap()};
    serde_json::from_str(&text).unwrap()
}
fn main(){
    let arguments=env::args().skip(1).collect::<Vec<_>>();
    assert_eq!(arguments.len(),3,"expected Documents path, file name and link name");
    let documents=PathBuf::from(&arguments[0]);let filename=&arguments[1];let linkname=&arguments[2];
    let address=format!("unix:path=/run/user/{}/bus",nix::unistd::geteuid());
    let connection=zbus::blocking::connection::Builder::address(address.as_str()).unwrap().method_timeout(Duration::from_secs(5)).build().unwrap();
    let files=Proxy::new(&connection,NAME,PATH,INTERFACE).unwrap();
    let proposal=value(&files,"ProposeRoots",None);let roots=proposal["data"]["roots"].as_array().unwrap();
    let selected=roots.iter().find(|root|root["display_path"]==documents.to_string_lossy().as_ref()).expect("Documents was not proposed");
    let root_id=selected["root_id"].as_str().unwrap();
    let open=json!({"schema_version":1,"root_id":root_id,"relative_path":filename,"access":"content"}).to_string();
    code(files.call::<_,_,String>("OpenScoped",&(open.as_str(),)).unwrap_err(),"PERMISSION_DENIED");
    let enroll=json!({"schema_version":1,"proposal_id":proposal["data"]["proposal_id"],"approved_root_ids":[root_id],"allowed_access":["content"],"confirmed":true}).to_string();
    let enrolled=value(&files,"EnrollRoots",Some(&enroll));assert_eq!(enrolled["data"]["roots"][0]["root_id"],root_id);
    let escape=json!({"schema_version":1,"root_id":root_id,"relative_path":linkname,"access":"content"}).to_string();
    code(files.call::<_,_,String>("OpenScoped",&(escape.as_str(),)).unwrap_err(),"PERMISSION_DENIED");
    let opened=value(&files,"OpenScoped",Some(&open));let handle=opened["data"]["file_handle"].as_str().unwrap();
    let read=json!({"schema_version":1,"file_handle":handle,"max_bytes":128}).to_string();let content=value(&files,"ReadScoped",Some(&read));
    assert_eq!(content["data"]["content"],"consented installed content");assert_eq!(content["data"]["bytes_read"],27);
    let revoke=json!({"schema_version":1,"root_id":root_id,"confirmed":true}).to_string();let revoked=value(&files,"RevokeRoot",Some(&revoke));
    assert_eq!(revoked["data"]["access_blocked"],true);assert_eq!(revoked["data"]["handles_revoked"],1);
    code(files.call::<_,_,String>("ReadScoped",&(read.as_str(),)).unwrap_err(),"PERMISSION_DENIED");
    println!("AIOS_INSTALLED_FILE_SCOPES_CLIENT={{\"preconsent_denied\":true,\"symlink_denied\":true,\"content_verified\":true,\"revocation_blocked\":true}}");
}
