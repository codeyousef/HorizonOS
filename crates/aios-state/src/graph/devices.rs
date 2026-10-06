//! Opaque native block-disk snapshots. Missing serial/WWN is unknown; boot-
//! scoped kernel locators never imply durable identity or permission to act.
use super::{native::{NativeTime,Error,Result},store::{GraphStore,Node,Scope,SourceTruth,ProviderSnapshot,ProviderState},ObservationTime,SourceRevision};
use aios_system::devices::{BlockDevice,BlockDevices};
use sha2::{Digest,Sha256};
pub const PROVIDER:&str="native-udev-block-disks";
fn key(device:&BlockDevice,boot:&super::BootId)->Result<(String,&'static str)>{
 let (identity,kind)=if let Some(wwn)=&device.wwn{(serde_json::json!(["udev-block-wwn",wwn]),"udev_wwn")}
 else if let Some(serial)=&device.serial{(serde_json::json!(["udev-block-serial",device.bus,serial]),"udev_serial")}
 else{(serde_json::json!(["boot-kernel-block",boot,device.devpath,device.major,device.minor]),"boot_scoped_kernel_locator")};
 let bytes=serde_json::to_vec(&identity).map_err(|_|Error::Clock)?;
 Ok((format!("device:{:x}",Sha256::digest(bytes)),kind))
}
pub struct NativeBlockSnapshot{captured:ObservationTime,inventory:BlockDevices,token:Option<String>}
impl NativeBlockSnapshot{
 pub fn collect(store:&GraphStore)->Result<Self>{
  if store.native_scope()!=Scope::System{return Err(Error::WrongScope);}
  let result=(||{let captured=NativeTime::observe()?.observation().clone();
   let token=store.reconciliation_plan(PROVIDER.into(),captured.clone(),SourceRevision::default())?.state.map(|s|s.token);
   let snapshot=Self{captured,inventory:aios_system::devices::read_block_devices()?,token};snapshot.fresh()?;Ok(snapshot)})();
  if result.is_err(){store.report_event_loss();}result
 }
 fn fresh(&self)->Result<()>{let now=NativeTime::observe()?.observation().clone();
  if now.boot!=self.captured.boot || now.monotonic_ns.checked_sub(self.captured.monotonic_ns).is_none_or(|n|n>2_000_000_000){return Err(Error::Expired);}Ok(())
 }
 pub fn inventory(&self)->&BlockDevices{&self.inventory}
 pub fn ids(&self)->Result<Vec<String>>{self.inventory.devices.iter().map(|d|key(d,&self.captured.boot).map(|v|v.0)).collect()}
 pub fn apply(&self,store:&GraphStore)->Result<ProviderState>{
  if store.native_scope()!=Scope::System{return Err(Error::WrongScope);}
  let result=(||{self.fresh()?;if aios_system::devices::read_block_devices()?!=self.inventory{return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged));}
   let mut nodes=Vec::new();let mut seen=std::collections::BTreeSet::new();
   for device in &self.inventory.devices{
    let (id,identity_kind)=key(device,&self.captured.boot)?;
    // Ambiguous stable properties cannot silently merge distinct devices.
    if !seen.insert(id.clone()){return Err(Error::Native(aios_protocol::contracts::ErrorCode::PartialResult));}
    nodes.push(Node{id:id.clone(),kind:"device".into(),scope:Scope::System,provider:PROVIDER.into(),stable_key:id,
     properties:serde_json::json!({"device":device,"identity_kind":identity_kind,"captured":self.captured,"execution_authority":false,
      "live_identity_retained":false,"other_hardware_classes_observed":false}),source_truth:SourceTruth::Running,realtime_ns:self.captured.realtime_ns});
   }
   self.fresh()?;let state=store.apply_provider_snapshot(ProviderSnapshot{provider:PROVIDER.into(),expected_token:self.token.clone(),source_truth:SourceTruth::Running,
    time:self.captured.clone(),source_revision:SourceRevision::default(),complete:self.inventory.complete,nodes,verified_absent_ids:vec![]})?;
   if aios_system::devices::read_block_devices()?!=self.inventory{return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged));}self.fresh()?;Ok(state)
  })();if result.is_err(){store.report_event_loss();}result
 }
}
#[cfg(test)]mod tests{
 use super::*;
 fn device()->BlockDevice{BlockDevice{syspath:"/sys/devices/fixture".into(),devpath:"/devices/fixture".into(),sysname:"fixture".into(),major:1,minor:2,initialized:true,serial:None,serial_short:None,wwn:None,bus:None,model:None,vendor:None}}
 #[test]fn missing_serial_stays_unknown_and_kernel_locator_is_boot_scoped(){
  let a=super::super::BootId::parse("11111111-1111-4111-8111-111111111111").unwrap();let b=super::super::BootId::parse("22222222-1111-4111-8111-111111111111").unwrap();
  let mut d=device();let first=key(&d,&a).unwrap();assert_eq!(first.1,"boot_scoped_kernel_locator");assert_ne!(first,key(&d,&b).unwrap());assert_eq!(d.serial,None);
  d.serial=Some("native-serial".into());assert_eq!(key(&d,&a).unwrap(),key(&d,&b).unwrap());assert_eq!(key(&d,&a).unwrap().1,"udev_serial");
  d.wwn=Some("native-wwn".into());assert_eq!(key(&d,&a).unwrap().1,"udev_wwn");let wwn=key(&d,&a).unwrap();d.serial=Some("other".into());assert_eq!(wwn,key(&d,&a).unwrap());
 }
}
