//! Fixed read-only session adapters for pinned PipeWire/WirePlumber and KDE
//! power services. Mutations remain behind approved task execution.
use aios_protocol::contracts::ErrorCode;
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
use std::{process::{Command,Stdio},thread,time::{Duration,Instant}};
use time::{OffsetDateTime,format_description::well_known::Rfc3339};

const PROVIDER_VERSION:&str=env!("CARGO_PKG_VERSION");
const WPCTL:&str=match option_env!("AIOS_WPCTL"){Some(path)=>path,None=>"wpctl"};
const KREADCONFIG:&str=match option_env!("AIOS_KREADCONFIG"){Some(path)=>path,None=>"kreadconfig6"};
fn envelope(provider:&str,data:Value,complete:bool)->Result<Value,ErrorCode>{
    let value=json!({"schema_version":1,"status":if complete{"ok"}else{"partial"},"observed_at":OffsetDateTime::now_utc().format(&Rfc3339).map_err(|_|ErrorCode::TargetChanged)?,
        "source":{"provider":provider,"provider_version":PROVIDER_VERSION},"evidence_ids":[],"complete":complete,"next_cursor":null,"data":data,"error":null});
    Ok(value)
}
fn command(program:&str,args:&[&str])->Result<String,ErrorCode>{
    let mut child=Command::new(program).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|_|ErrorCode::UnsupportedCapability)?;
    let deadline=Instant::now()+Duration::from_secs(2);
    loop{match child.try_wait().map_err(|_|ErrorCode::UnsupportedCapability)?{Some(status)=>{
        let output=child.wait_with_output().map_err(|_|ErrorCode::UnsupportedCapability)?;
        if !status.success()||output.stdout.len()>256*1024||output.stderr.len()>64*1024{return Err(ErrorCode::UnsupportedCapability);}
        return String::from_utf8(output.stdout).map_err(|_|ErrorCode::TargetChanged);
    },None if Instant::now()<deadline=>thread::sleep(Duration::from_millis(10)),None=>{let _=child.kill();let _=child.wait();return Err(ErrorCode::DeadlineExceeded)}}}
}
#[derive(Clone,Debug,PartialEq,Eq)]struct AudioNode{direction:&'static str,name:String,muted:Option<bool>,default:bool}
fn handle(direction:&str,name:&str)->String{format!("audio:{direction}:{:x}",Sha256::digest(name.as_bytes()))}
fn parse_audio(value:&str)->Result<Vec<AudioNode>,ErrorCode>{
    if value.len()>256*1024{return Err(ErrorCode::ResourceExhausted);}let mut section=None;let mut result=Vec::new();
    for line in value.lines(){let trimmed=line.trim();match trimmed{
        "├─ Sinks:"|"└─ Sinks:"=>{section=Some("output");continue},
        "├─ Sources:"|"└─ Sources:"=>{section=Some("input");continue},
        _ if trimmed.starts_with("├─ Filters:")||trimmed.starts_with("└─ Filters:")||trimmed.starts_with("├─ Streams:")||trimmed.starts_with("└─ Streams:")||trimmed=="Video"=>{section=None;continue},_=>{}
    }
        let Some(direction)=section else{continue};let item=trimmed.trim_start_matches('│').trim();if item.is_empty(){continue;}
        let (default,item)=if let Some(rest)=item.strip_prefix('*'){(true,rest.trim())}else{(false,item)};
        let Some((number,rest))=item.split_once('.') else{continue};if number.trim().parse::<u32>().is_err(){continue;}
        let name=rest.split_once('[').map_or(rest,|v|v.0).trim();if name.is_empty()||name.len()>256{return Err(ErrorCode::TargetChanged);}
        let muted=if rest.contains("[MUTED]"){Some(true)}else if rest.contains("[vol:"){Some(false)}else{None};
        result.push(AudioNode{direction,name:name.into(),muted,default});if result.len()>100{return Err(ErrorCode::ResourceExhausted);}
    }
    Ok(result)
}
fn audio_nodes()->Result<Vec<AudioNode>,ErrorCode>{parse_audio(&command(WPCTL,&["status","--name"])?)}
fn audio_list(direction:&str)->Result<Value,ErrorCode>{
    let nodes=audio_nodes()?;let devices=nodes.into_iter().filter(|n|n.direction==direction).map(|n|json!({"node_id":handle(n.direction,&n.name),"name":n.name,"available":true,"muted":n.muted,"default":n.default,"routing":"pipewire-session-default"})).collect::<Vec<_>>();
    envelope("pipewire-wireplumber",json!({"devices":devices}),true)
}
fn audio_default(direction:&str)->Result<Value,ErrorCode>{
    if !matches!(direction,"input"|"output"){return Err(ErrorCode::InvalidArgument);}let nodes=audio_nodes()?;let node=nodes.iter().find(|n|n.direction==direction&&n.default);
    envelope("pipewire-wireplumber",json!({"direction":direction,"node_id":node.map(|n|handle(direction,&n.name)),"available":node.is_some()}),true)
}
fn setting(key:&str)->Result<Value,ErrorCode>{
    let (value,provider)=match key{
        "desktop.theme_mode"=>{
            let value=command(KREADCONFIG,&["--file","kdeglobals","--group","General","--key","ColorScheme"])?;
            let mode=match value.trim(){"BreezeLight"=>"light","BreezeDark"=>"dark",_=>return Err(ErrorCode::UnsupportedCapability)};
            (json!(mode),"kconfig-pinned")
        },
        "display.idle_seconds"=>{
            let session=zbus::blocking::Connection::session().map_err(|_|ErrorCode::UnsupportedCapability)?;
            let power=zbus::blocking::Proxy::new(&session,"org.kde.Solid.PowerManagement","/org/kde/Solid/PowerManagement","org.kde.Solid.PowerManagement").map_err(|_|ErrorCode::UnsupportedCapability)?;
            let profile:String=power.call("currentProfile",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
            if !matches!(profile.as_str(),"AC"|"Battery"|"LowBattery"){return Err(ErrorCode::UnsupportedCapability);}
            let value=command(KREADCONFIG,&["--file","powerdevilrc","--group",&profile,"--group","Display","--key","TurnOffDisplayIdleTimeoutSec"])?;
            let seconds=value.trim().parse::<u64>().map_err(|_|ErrorCode::UnsupportedCapability)?;
            if !(60..=3600).contains(&seconds){return Err(ErrorCode::UnsupportedCapability);}
            (json!(seconds),"powerdevil-kconfig-pinned")
        },
        "keyboard.backlight_percent"=>{
            let system=zbus::blocking::Connection::system().map_err(|_|ErrorCode::UnsupportedCapability)?;
            let keyboard=zbus::blocking::Proxy::new(&system,"org.freedesktop.UPower","/org/freedesktop/UPower/KbdBacklight","org.freedesktop.UPower.KbdBacklight").map_err(|_|ErrorCode::UnsupportedCapability)?;
            let current:i32=keyboard.call("GetBrightness",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
            let maximum:i32=keyboard.call("GetMaxBrightness",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
            if maximum<=0||current<0||current>maximum{return Err(ErrorCode::TargetChanged);}
            (json!(((i64::from(current)*100+i64::from(maximum)/2)/i64::from(maximum)) as u64),"upower-kbd-backlight")
        },
        _=>return Err(ErrorCode::UnsupportedCapability),
    };
    envelope(provider,json!({"key":key,"value":value,"ownership":"user"}),true)
}
fn power_status()->Result<Value,ErrorCode>{
    let session=zbus::blocking::Connection::session().map_err(|_|ErrorCode::UnsupportedCapability)?;
    let profile=zbus::blocking::Proxy::new(&session,"org.kde.Solid.PowerManagement","/org/kde/Solid/PowerManagement/Actions/PowerProfile","org.kde.Solid.PowerManagement.Actions.PowerProfile").map_err(|_|ErrorCode::UnsupportedCapability)?;
    let current:String=profile.call("currentProfile",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
    let choices:Vec<String>=profile.call("profileChoices",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
    if choices.len()>3||choices.iter().any(|v|!matches!(v.as_str(),"power-saver"|"balanced"|"performance")){return Err(ErrorCode::TargetChanged);}
    let current=choices.contains(&current).then_some(current);let system=zbus::blocking::Connection::system().map_err(|_|ErrorCode::UnsupportedCapability)?;
    let upower=zbus::blocking::Proxy::new(&system,"org.freedesktop.UPower","/org/freedesktop/UPower","org.freedesktop.UPower").map_err(|_|ErrorCode::UnsupportedCapability)?;
    let on_battery:bool=upower.get_property("OnBattery").map_err(|_|ErrorCode::UnsupportedCapability)?;
    let devices:Vec<zbus::zvariant::OwnedObjectPath>=upower.call("EnumerateDevices",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
    let mut battery=None;
    for path in devices.iter().take(32){let device=zbus::blocking::Proxy::new(&system,"org.freedesktop.UPower",path.as_str(),"org.freedesktop.UPower.Device").map_err(|_|ErrorCode::UnsupportedCapability)?;let kind:u32=device.get_property("Type").map_err(|_|ErrorCode::UnsupportedCapability)?;if kind==2{let value:f64=device.get_property("Percentage").map_err(|_|ErrorCode::UnsupportedCapability)?;if !value.is_finite()||!(0.0..=100.0).contains(&value){return Err(ErrorCode::TargetChanged);}battery=Some(value.round() as u64);break}}
    let mut unsupported=Vec::new();if battery.is_none(){unsupported.push("battery_percent");}if current.is_none(){unsupported.push("profile");}
    envelope("upower-powerdevil",json!({"on_ac":!on_battery,"battery_percent":battery,"profile":current,"available_profiles":choices,"unsupported_fields":unsupported}),unsupported.is_empty())
}
pub fn invoke(action:&aios_protocol::contracts::Action)->Result<Value,ErrorCode>{
    let value=match action.action_id(){"audio.outputs"=>audio_list("output")?,"audio.inputs"=>audio_list("input")?,"audio.default_get"=>audio_default(action.arguments_value().get("direction").and_then(Value::as_str).ok_or(ErrorCode::InvalidArgument)?)?,"power.status"=>power_status()?,"settings.get"=>setting(action.arguments_value().get("key").and_then(Value::as_str).ok_or(ErrorCode::InvalidArgument)?)?,_=>return Err(ErrorCode::UnsupportedCapability)};
    aios_protocol::validation::validate_result(action.action_id(),serde_json::to_string(&value).map_err(|_|ErrorCode::TargetChanged)?.as_bytes())?;Ok(value)
}
pub fn available_actions(actions:&[String])->Vec<String>{
    let audio=actions.iter().any(|id|id.starts_with("audio.")).then(||audio_nodes().is_ok());
    let settings=actions.iter().any(|id|id=="settings.get").then(||["desktop.theme_mode","display.idle_seconds","keyboard.backlight_percent"].iter().any(|key|setting(key).is_ok()));
    let power=actions.iter().any(|id|id=="power.status").then(||power_status().is_ok());
    actions.iter().filter(|id|match id.as_str(){
        "audio.outputs"|"audio.inputs"|"audio.default_get"=>audio==Some(true),
        "settings.get"=>settings==Some(true),
        "power.status"=>power==Some(true),
        _=>false,
    }).cloned().collect()
}

#[cfg(test)]mod tests{use super::*;#[test]fn pinned_wpctl_shape_yields_stable_handles_and_missing_input(){let value="Audio\n ├─ Sinks:\n │  *   52. auto_null [vol: 1.00]\n ├─ Sources:\n │  \n ├─ Filters:\n";let nodes=parse_audio(value).unwrap();assert_eq!(nodes.len(),1);assert!(nodes[0].default);assert_eq!(nodes[0].muted,Some(false));assert_eq!(handle("output","auto_null"),handle("output","auto_null"));}#[test]fn malformed_or_unbounded_audio_is_refused(){assert!(parse_audio("Audio\n ├─ Sinks:\n │ * x. invalid [vol: 1.00]").unwrap().is_empty());assert_eq!(parse_audio(&"x".repeat(256*1024+1)),Err(ErrorCode::ResourceExhausted));}}
