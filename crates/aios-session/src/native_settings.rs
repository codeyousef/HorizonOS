//! Fixed session adapters for pinned PipeWire/WirePlumber, KDE and UPower
//! providers. Mutations remain behind bounded task execution.
use aios_protocol::contracts::ErrorCode;
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
use std::{process::{Command,Stdio},thread,time::{Duration,Instant}};
use time::{OffsetDateTime,format_description::well_known::Rfc3339};

const PROVIDER_VERSION:&str=env!("CARGO_PKG_VERSION");
const WPCTL:&str=match option_env!("AIOS_WPCTL"){Some(path)=>path,None=>"wpctl"};
const KREADCONFIG:&str=match option_env!("AIOS_KREADCONFIG"){Some(path)=>path,None=>"kreadconfig6"};
const SYSTEMCTL:&str=match option_env!("AIOS_SYSTEMCTL"){Some(path)=>path,None=>"systemctl"};
fn session_bus()->Result<zbus::blocking::Connection,ErrorCode>{
    let address=format!("unix:path=/run/user/{}/bus",nix::unistd::geteuid().as_raw());
    zbus::blocking::connection::Builder::address(address.as_str()).map_err(|_|ErrorCode::UnsupportedCapability)?
        .method_timeout(Duration::from_millis(500)).build().map_err(|_|ErrorCode::UnsupportedCapability)
}
fn system_bus()->Result<zbus::blocking::Connection,ErrorCode>{
    zbus::blocking::connection::Builder::address("unix:path=/run/dbus/system_bus_socket").map_err(|_|ErrorCode::UnsupportedCapability)?
        .method_timeout(Duration::from_millis(500)).build().map_err(|_|ErrorCode::UnsupportedCapability)
}
fn projected_setting(name:&str)->String{format!("/run/user/{}/aios/desktop-settings/{name}",nix::unistd::geteuid().as_raw())}
fn envelope(provider:&str,data:Value,complete:bool)->Result<Value,ErrorCode>{
    let error=if complete{Value::Null}else{json!({"code":"PARTIAL_RESULT","message":"One or more registered provider fields are unsupported","retryable":false})};
    let value=json!({"schema_version":1,"status":if complete{"ok"}else{"partial"},"observed_at":OffsetDateTime::now_utc().format(&Rfc3339).map_err(|_|ErrorCode::TargetChanged)?,
        "source":{"provider":provider,"provider_version":PROVIDER_VERSION},"evidence_ids":[],"complete":complete,"next_cursor":null,"data":data,"error":error});
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
#[derive(Clone,Debug,PartialEq,Eq)]struct AudioNode{id:u32,direction:&'static str,name:String,muted:Option<bool>,default:bool}
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
        let Some((number,rest))=item.split_once('.') else{continue};let Ok(id)=number.trim().parse::<u32>() else{continue};
        let name=rest.split_once('[').map_or(rest,|v|v.0).trim();if name.is_empty()||name.len()>256{return Err(ErrorCode::TargetChanged);}
        let muted=if rest.contains(" MUTED]")||rest.contains("[MUTED]"){Some(true)}else if rest.contains("[vol:"){Some(false)}else{None};
        if result.iter().any(|node: &AudioNode|node.direction==direction&&node.name==name){return Err(ErrorCode::TargetChanged);}
        result.push(AudioNode{id,direction,name:name.into(),muted,default});if result.len()>100{return Err(ErrorCode::ResourceExhausted);}
    }
    Ok(result)
}
fn node_identity(node:&AudioNode)->String{
    format!("{:x}",Sha256::digest(format!("{}\0{}",node.direction,node.name).as_bytes()))
}
fn selected_node(reference:&str,direction:Option<&str>)->Result<AudioNode,ErrorCode>{
    let nodes=audio_nodes()?;let mut matches=nodes.into_iter().filter(|node|handle(node.direction,&node.name)==reference&&direction.is_none_or(|value|value==node.direction));
    let node=matches.next().ok_or(ErrorCode::TargetNotFound)?;if matches.next().is_some(){return Err(ErrorCode::TargetChanged);}Ok(node)
}
fn audio_nodes()->Result<Vec<AudioNode>,ErrorCode>{parse_audio(&command(WPCTL,&["status","--name"])?)}
fn await_node(reference:&str,direction:Option<&str>,matches:impl Fn(&AudioNode)->bool)->Result<AudioNode,ErrorCode>{
    let deadline=Instant::now()+Duration::from_secs(1);
    loop{
        let node=selected_node(reference,direction)?;
        if matches(&node){return Ok(node);}
        if Instant::now()>=deadline{return Err(ErrorCode::PartialResult);}
        thread::sleep(Duration::from_millis(20));
    }
}
fn audio_list(direction:&str)->Result<Value,ErrorCode>{
    let nodes=audio_nodes()?;let devices=nodes.into_iter().filter(|n|n.direction==direction).map(|n|json!({"node_id":handle(n.direction,&n.name),"name":n.name,"available":true,"muted":n.muted,"default":n.default,"routing":"pipewire-session-default"})).collect::<Vec<_>>();
    envelope("pipewire-wireplumber",json!({"devices":devices}),true)
}
fn audio_default(direction:&str)->Result<Value,ErrorCode>{
    if !matches!(direction,"input"|"output"){return Err(ErrorCode::InvalidArgument);}let nodes=audio_nodes()?;let node=nodes.iter().find(|n|n.direction==direction&&n.default);
    envelope("pipewire-wireplumber",json!({"direction":direction,"node_id":node.map(|n|handle(direction,&n.name)),"available":node.is_some()}),true)
}
fn keyboard_state()->Result<(i32,i32,u64),ErrorCode>{
    let system=system_bus()?;
    let keyboard=zbus::blocking::Proxy::new(&system,"org.freedesktop.UPower","/org/freedesktop/UPower/KbdBacklight","org.freedesktop.UPower.KbdBacklight").map_err(|_|ErrorCode::UnsupportedCapability)?;
    let current:i32=keyboard.call("GetBrightness",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
    let maximum:i32=keyboard.call("GetMaxBrightness",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
    if maximum<=0||current<0||current>maximum{return Err(ErrorCode::TargetChanged);}
    let percent=((i64::from(current)*100+i64::from(maximum)/2)/i64::from(maximum)) as u64;
    Ok((current,maximum,percent))
}
fn brightness_for_percent(percent:u64,maximum:i32)->Result<i32,ErrorCode>{
    let brightness=((percent*i64::from(maximum) as u64+50)/100) as i32;
    let represented=((i64::from(brightness)*100+i64::from(maximum)/2)/i64::from(maximum)) as u64;
    if represented!=percent{return Err(ErrorCode::UnsupportedCapability);}Ok(brightness)
}
fn idle_state()->Result<(String,u64),ErrorCode>{
    let session=session_bus()?;
    let power=zbus::blocking::Proxy::new(&session,"org.kde.Solid.PowerManagement","/org/kde/Solid/PowerManagement","org.kde.Solid.PowerManagement").map_err(|_|ErrorCode::UnsupportedCapability)?;
    let profile:String=power.call("currentProfile",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
    if !matches!(profile.as_str(),"AC"|"Battery"|"LowBattery"){return Err(ErrorCode::UnsupportedCapability);}
    let file=projected_setting("powerdevilrc");
    let value=command(KREADCONFIG,&["--file",&file,"--group",&profile,"--group","Display","--key","TurnOffDisplayIdleTimeoutSec"])?;
    let seconds=value.trim().parse::<u64>().map_err(|_|ErrorCode::UnsupportedCapability)?;
    if !(60..=3600).contains(&seconds){return Err(ErrorCode::UnsupportedCapability);}
    Ok((profile,seconds))
}
fn setting(key:&str)->Result<Value,ErrorCode>{
    let (value,provider)=match key{
        "desktop.theme_mode"=>{
            let file=projected_setting("kdeglobals");
            let value=command(KREADCONFIG,&["--file",&file,"--group","General","--key","ColorScheme"])?;
            let mode=match value.trim(){"BreezeLight"=>"light","BreezeDark"=>"dark",_=>return Err(ErrorCode::UnsupportedCapability)};
            (json!(mode),"kconfig-pinned")
        },
        "display.idle_seconds"=>{
            let (_,seconds)=idle_state()?;
            (json!(seconds),"powerdevil-kconfig-pinned")
        },
        "keyboard.backlight_percent"=>{
            let (_,_,percent)=keyboard_state()?;
            (json!(percent),"upower-kbd-backlight")
        },
        _=>return Err(ErrorCode::UnsupportedCapability),
    };
    envelope(provider,json!({"key":key,"value":value,"ownership":"user"}),true)
}
fn setting_value(key:&str)->Result<Value,ErrorCode>{
    setting(key)?.get("data").and_then(|value|value.get("value")).cloned().ok_or(ErrorCode::TargetChanged)
}
fn power_status()->Result<Value,ErrorCode>{
    let session=session_bus()?;
    let profile=zbus::blocking::Proxy::new(&session,"org.kde.Solid.PowerManagement","/org/kde/Solid/PowerManagement/Actions/PowerProfile","org.kde.Solid.PowerManagement.Actions.PowerProfile").map_err(|_|ErrorCode::UnsupportedCapability)?;
    let current:String=profile.call("currentProfile",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
    let choices:Vec<String>=profile.call("profileChoices",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
    if choices.len()>3||choices.iter().any(|v|!matches!(v.as_str(),"power-saver"|"balanced"|"performance")){return Err(ErrorCode::TargetChanged);}
    let current=choices.contains(&current).then_some(current);let system=system_bus()?;
    let upower=zbus::blocking::Proxy::new(&system,"org.freedesktop.UPower","/org/freedesktop/UPower","org.freedesktop.UPower").map_err(|_|ErrorCode::UnsupportedCapability)?;
    let on_battery:bool=upower.get_property("OnBattery").map_err(|_|ErrorCode::UnsupportedCapability)?;
    let devices:Vec<zbus::zvariant::OwnedObjectPath>=upower.call("EnumerateDevices",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
    let mut battery=None;
    for path in devices.iter().take(32){let device=zbus::blocking::Proxy::new(&system,"org.freedesktop.UPower",path.as_str(),"org.freedesktop.UPower.Device").map_err(|_|ErrorCode::UnsupportedCapability)?;let kind:u32=device.get_property("Type").map_err(|_|ErrorCode::UnsupportedCapability)?;if kind==2{let value:f64=device.get_property("Percentage").map_err(|_|ErrorCode::UnsupportedCapability)?;if !value.is_finite()||!(0.0..=100.0).contains(&value){return Err(ErrorCode::TargetChanged);}battery=Some(value.round() as u64);break}}
    let mut unsupported=Vec::new();if battery.is_none(){unsupported.push("battery_percent");}if current.is_none(){unsupported.push("profile");}
    envelope("upower-powerdevil",json!({"on_ac":!on_battery,"battery_percent":battery,"profile":current,"available_profiles":choices,"unsupported_fields":unsupported}),unsupported.is_empty())
}
fn power_profile_state()->Result<(String,Vec<String>),ErrorCode>{
    let session=session_bus()?;
    let profile=zbus::blocking::Proxy::new(&session,"org.kde.Solid.PowerManagement","/org/kde/Solid/PowerManagement/Actions/PowerProfile","org.kde.Solid.PowerManagement.Actions.PowerProfile").map_err(|_|ErrorCode::UnsupportedCapability)?;
    let current:String=profile.call("currentProfile",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
    let choices:Vec<String>=profile.call("profileChoices",&()).map_err(|_|ErrorCode::UnsupportedCapability)?;
    if choices.is_empty()||choices.len()>3||choices.iter().any(|value|!matches!(value.as_str(),"power-saver"|"balanced"|"performance")){return Err(ErrorCode::UnsupportedCapability);}
    if !choices.contains(&current){return Err(ErrorCode::TargetChanged);}Ok((current,choices))
}
fn await_power_profile(expected:&str)->Result<(),ErrorCode>{
    let deadline=Instant::now()+Duration::from_secs(2);
    loop{
        if power_profile_state()?.0==expected{return Ok(());}
        if Instant::now()>=deadline{return Err(ErrorCode::PartialResult);}
        thread::sleep(Duration::from_millis(20));
    }
}
enum MutationKind{
    AudioDefault{direction:String,target:AudioNode,prior:AudioNode},
    AudioMute{target:AudioNode,muted:bool,prior:bool},
    Theme{value:String,prior:String},
    Backlight{value:u64,prior:u64,prior_brightness:i32,maximum:i32},
    Idle{value:u64,prior:u64,profile:String},
    PowerProfile{value:String,prior:String,choices:Vec<String>},
}
pub struct PreparedMutation{action_id:String,arguments:Value,operation_id:String,kind:MutationKind}
pub struct MutationReceipt{pub output:Value,pub recovery:Value,pub changed:bool}
impl PreparedMutation{
    pub fn prepare(action:&aios_protocol::contracts::Action)->Result<Self,ErrorCode>{
        let arguments=action.arguments_value();let operation_id=uuid::Uuid::new_v4().to_string();
        let kind=match action.action_id(){
            "audio.default_set"=>{
                let direction=arguments.get("direction").and_then(Value::as_str).ok_or(ErrorCode::InvalidArgument)?;
                let reference=arguments.get("node_id").and_then(Value::as_str).ok_or(ErrorCode::InvalidArgument)?;
                let target=selected_node(reference,Some(direction))?;let nodes=audio_nodes()?;
                let prior=nodes.into_iter().find(|node|node.direction==direction&&node.default).ok_or(ErrorCode::UnsupportedCapability)?;
                MutationKind::AudioDefault{direction:direction.into(),target,prior}
            },
            "audio.mute_set"=>{
                let reference=arguments.get("node_id").and_then(Value::as_str).ok_or(ErrorCode::InvalidArgument)?;
                let muted=arguments.get("muted").and_then(Value::as_bool).ok_or(ErrorCode::InvalidArgument)?;
                let target=selected_node(reference,None)?;let prior=target.muted.ok_or(ErrorCode::UnsupportedCapability)?;
                MutationKind::AudioMute{target,muted,prior}
            },
            "settings.set"=>{
                let key=arguments.get("key").and_then(Value::as_str).ok_or(ErrorCode::InvalidArgument)?;
                match key{
                    "desktop.theme_mode"=>{
                        let value=arguments.get("value").and_then(Value::as_str).ok_or(ErrorCode::InvalidArgument)?;
                        if !matches!(value,"light"|"dark"){return Err(ErrorCode::InvalidArgument);}
                        let prior=setting_value(key)?.as_str().ok_or(ErrorCode::TargetChanged)?.to_owned();
                        MutationKind::Theme{value:value.into(),prior}
                    },
                    "keyboard.backlight_percent"=>{
                        let value=arguments.get("value").and_then(Value::as_u64).filter(|value|*value<=100).ok_or(ErrorCode::InvalidArgument)?;
                        let (prior_brightness,maximum,prior)=keyboard_state()?;
                        brightness_for_percent(prior,maximum)?;brightness_for_percent(value,maximum)?;
                        MutationKind::Backlight{value,prior,prior_brightness,maximum}
                    },
                    "display.idle_seconds"=>{
                        let value=arguments.get("value").and_then(Value::as_u64).filter(|value|(60..=3600).contains(value)).ok_or(ErrorCode::InvalidArgument)?;
                        let (profile,prior)=idle_state()?;
                        MutationKind::Idle{value,prior,profile}
                    },
                    _=>return Err(ErrorCode::UnsupportedCapability),
                }
            },
            "power.profile_set"=>{
                let value=arguments.get("profile").and_then(Value::as_str).filter(|value|matches!(*value,"power-saver"|"balanced"|"performance")).ok_or(ErrorCode::InvalidArgument)?;
                let (prior,choices)=power_profile_state()?;
                if !choices.iter().any(|choice|choice==value){return Err(ErrorCode::UnsupportedCapability);}
                MutationKind::PowerProfile{value:value.into(),prior,choices}
            },
            _=>return Err(ErrorCode::UnsupportedCapability),
        };
        Ok(Self{action_id:action.action_id().into(),arguments,operation_id,kind})
    }
    pub fn scope(&self)->aios_policy::Scope{
        let resource=match &self.kind{
            MutationKind::AudioDefault{target,..}|MutationKind::AudioMute{target,..}=>Some(aios_policy::Resource{
                field:"node_id".into(),kind:"scope-owner-expiry".into(),handle:handle(target.direction,&target.name),identity_sha256:node_identity(target)}),
            MutationKind::Theme{..}|MutationKind::Backlight{..}|MutationKind::Idle{..}|MutationKind::PowerProfile{..}=>None,
        };
        aios_policy::Scope{actions:[self.action_id.clone()].into(),resources:resource.into_iter().collect(),..Default::default()}
    }
    pub fn execute(mut self,mut revalidate:impl FnMut(&Self)->Result<(),ErrorCode>)->Result<MutationReceipt,ErrorCode>{
        revalidate(&self)?;
        let (output,recovery,changed)=match &self.kind{
            MutationKind::AudioDefault{direction,target,prior}=>{
                let changed=!target.default;if changed{let id=target.id.to_string();command(WPCTL,&["set-default",&id])?;}
                revalidate(&self)?;let reference=handle(target.direction,&target.name);
                await_node(&reference,Some(direction),|node|node.default)?;
                (json!({"direction":direction,"node_id":handle(target.direction,&target.name),"available":true}),
                    json!({"kind":"tool_call","action_id":"audio.default_set","arguments":{"direction":direction,"node_id":handle(prior.direction,&prior.name)}}),changed)
            },
            MutationKind::AudioMute{target,muted,prior}=>{
                let changed=*prior!=*muted;if changed{let id=target.id.to_string();let value=if *muted{"1"}else{"0"};command(WPCTL,&["set-mute",&id,value])?;}
                revalidate(&self)?;let reference=handle(target.direction,&target.name);
                await_node(&reference,None,|node|node.muted==Some(*muted))?;
                (json!({"node_id":handle(target.direction,&target.name),"muted":muted}),
                    json!({"kind":"tool_call","action_id":"audio.mute_set","arguments":{"node_id":handle(target.direction,&target.name),"muted":prior}}),changed)
            },
            MutationKind::Theme{value,prior}=>{
                if setting_value("desktop.theme_mode")?!=json!(prior){return Err(ErrorCode::TargetChanged);}
                let changed=value!=prior;if changed{
                    let scheme=match value.as_str(){"light"=>"BreezeLight","dark"=>"BreezeDark",_=>return Err(ErrorCode::InvalidArgument)};
                    let unit=format!("aios-setting-theme@{scheme}.service");command(SYSTEMCTL,&["--user","start","--wait",&unit])?;
                    command(SYSTEMCTL,&["--user","start","--wait","aios-sessiond-settings-sync.service"])?;
                }
                revalidate(&self)?;if setting_value("desktop.theme_mode")?!=json!(value){return Err(ErrorCode::PartialResult);}
                (json!({"key":"desktop.theme_mode","value":value,"ownership":"user"}),
                    json!({"kind":"tool_call","action_id":"settings.set","arguments":{"key":"desktop.theme_mode","value":prior}}),changed)
            },
            MutationKind::Backlight{value,prior,prior_brightness,maximum}=>{
                let (current,current_maximum,_)=keyboard_state()?;
                if current!=*prior_brightness||current_maximum!=*maximum{return Err(ErrorCode::TargetChanged);}
                let target=brightness_for_percent(*value,*maximum)?;let changed=current!=target;
                if changed{
                    let system=system_bus()?;
                    let keyboard=zbus::blocking::Proxy::new(&system,"org.freedesktop.UPower","/org/freedesktop/UPower/KbdBacklight","org.freedesktop.UPower.KbdBacklight").map_err(|_|ErrorCode::UnsupportedCapability)?;
                    let _:()=keyboard.call("SetBrightness",&target).map_err(|_|ErrorCode::PermissionDenied)?;
                }
                revalidate(&self)?;let (actual,actual_maximum,percent)=keyboard_state()?;
                if actual!=target||actual_maximum!=*maximum||percent!=*value{return Err(ErrorCode::PartialResult);}
                (json!({"key":"keyboard.backlight_percent","value":value,"ownership":"user"}),
                    json!({"kind":"tool_call","action_id":"settings.set","arguments":{"key":"keyboard.backlight_percent","value":prior}}),changed)
            },
            MutationKind::Idle{value,prior,profile}=>{
                let (current_profile,current)=idle_state()?;
                if current_profile!=*profile||current!=*prior{return Err(ErrorCode::TargetChanged);}
                let changed=value!=prior;if changed{
                    let unit=format!("aios-setting-idle-{profile}@{value}.service");command(SYSTEMCTL,&["--user","start","--wait",&unit])?;
                    let session=session_bus()?;
                    let power=zbus::blocking::Proxy::new(&session,"org.kde.Solid.PowerManagement","/org/kde/Solid/PowerManagement","org.kde.Solid.PowerManagement").map_err(|_|ErrorCode::UnsupportedCapability)?;
                    let _:()=power.call("reparseConfiguration",&()).map_err(|_|ErrorCode::PartialResult)?;
                    command(SYSTEMCTL,&["--user","start","--wait","aios-sessiond-settings-sync.service"])?;
                }
                revalidate(&self)?;let (actual_profile,actual)=idle_state()?;
                if actual_profile!=*profile||actual!=*value{return Err(ErrorCode::PartialResult);}
                (json!({"key":"display.idle_seconds","value":value,"ownership":"user"}),
                    json!({"kind":"tool_call","action_id":"settings.set","arguments":{"key":"display.idle_seconds","value":prior}}),changed)
            },
            MutationKind::PowerProfile{value,prior,choices}=>{
                let (current,current_choices)=power_profile_state()?;
                if current!=*prior||current_choices!=*choices{return Err(ErrorCode::TargetChanged);}
                let changed=value!=prior;if changed{
                    let session=session_bus()?;
                    let profile=zbus::blocking::Proxy::new(&session,"org.kde.Solid.PowerManagement","/org/kde/Solid/PowerManagement/Actions/PowerProfile","org.kde.Solid.PowerManagement.Actions.PowerProfile").map_err(|_|ErrorCode::UnsupportedCapability)?;
                    let _:()=profile.call("setProfile",&value.as_str()).map_err(|_|ErrorCode::PermissionDenied)?;
                }
                revalidate(&self)?;await_power_profile(value)?;
                (json!({"profile":value}),json!({"kind":"tool_call","action_id":"power.profile_set","arguments":{"profile":prior}}),changed)
            },
        };
        let provider=match &self.kind{MutationKind::Theme{..}=>"plasma-colorscheme",MutationKind::Backlight{..}=>"upower-kbd-backlight",MutationKind::Idle{..}=>"powerdevil-kconfig-pinned",MutationKind::PowerProfile{..}=>"powerdevil-powerprofiles",_=>"pipewire-wireplumber"};
        let result=envelope(provider,{
            let mut data=output;let map=data.as_object_mut().ok_or(ErrorCode::TargetChanged)?;
            map.insert("operation_id".into(),json!(self.operation_id));map.insert("transaction_id".into(),Value::Null);
            map.insert("verification".into(),json!({"outcome":"verified","check_id":"native_readback","evidence_ids":[],"explanation":"Provider readback matched the requested value"}));data
        },true)?;
        aios_protocol::validation::validate_result(&self.action_id,serde_json::to_string(&result).map_err(|_|ErrorCode::TargetChanged)?.as_bytes())?;
        Ok(MutationReceipt{output:result,recovery,changed})
    }
}
impl aios_policy::CurrentResources for PreparedMutation{
    fn resolve(&self,field:&str,kind:&str,reference:&str)->Result<String,ErrorCode>{
        if field!="node_id"||kind!="scope-owner-expiry"{return Err(ErrorCode::PermissionDenied);}
        let node=selected_node(reference,None)?;Ok(node_identity(&node))
    }
    fn dynamic_arguments(&self,id:&str,args:&Value,scope:&aios_policy::Scope)->Result<(),ErrorCode>{
        if id==self.action_id&&args==&self.arguments&&scope.actions.contains(id){Ok(())}else{Err(ErrorCode::PermissionDenied)}
    }
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
        "audio.outputs"|"audio.inputs"|"audio.default_get"|"audio.default_set"|"audio.mute_set"=>audio==Some(true),
        "settings.get"|"settings.set"=>settings==Some(true),
        "power.status"=>power==Some(true),
        _=>false,
    }).cloned().collect()
}

#[cfg(test)]mod tests{use super::*;#[test]fn pinned_wpctl_shape_yields_stable_handles_and_mute_state(){let value="Audio\n ├─ Sinks:\n │  *   52. auto_null [vol: 1.00]\n ├─ Sources:\n │  \n ├─ Filters:\n";let nodes=parse_audio(value).unwrap();assert_eq!(nodes.len(),1);assert!(nodes[0].default);assert_eq!(nodes[0].muted,Some(false));assert_eq!(handle("output","auto_null"),handle("output","auto_null"));let muted=parse_audio("Audio\n ├─ Sinks:\n │  *   52. auto_null [vol: 1.00 MUTED]\n ├─ Sources:\n").unwrap();assert_eq!(muted[0].muted,Some(true));}#[test]fn malformed_unbounded_or_colliding_audio_is_refused(){assert!(parse_audio("Audio\n ├─ Sinks:\n │ * x. invalid [vol: 1.00]").unwrap().is_empty());assert_eq!(parse_audio(&"x".repeat(256*1024+1)),Err(ErrorCode::ResourceExhausted));assert_eq!(parse_audio("Audio\n ├─ Sinks:\n │ 1. duplicate [vol: 1.00]\n │ 2. duplicate [vol: 1.00]"),Err(ErrorCode::TargetChanged));}#[test]fn backlight_refuses_percentages_that_cannot_be_recovered_exactly(){assert_eq!(brightness_for_percent(37,100),Ok(37));assert_eq!(brightness_for_percent(100,1),Ok(1));assert_eq!(brightness_for_percent(1,1),Err(ErrorCode::UnsupportedCapability));}#[test]fn partial_native_observations_have_a_contract_valid_error(){let value=envelope("fixture",json!({"on_ac":true,"battery_percent":null,"profile":null,"available_profiles":[],"unsupported_fields":["battery_percent","profile"]}),false).unwrap();assert_eq!(value["error"]["code"],"PARTIAL_RESULT");assert!(aios_protocol::validation::validate_result("power.status",value.to_string().as_bytes()).is_ok());}}
