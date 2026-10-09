use crate::identity::Peer;
use aios_protocol::{MAX_FRAME_BYTES, MAX_TASK_BYTES, contracts::ErrorCode};
use serde_json::{Value, json};
use std::{collections::HashMap, time::Duration};
use zbus::blocking::{Connection, Proxy};

const NAME: &str = "org.aios.Executor1";
const PATH: &str = "/org/aios/Executor1";
const INTERFACE: &str = "org.aios.Executor1";

#[derive(Clone, Debug, PartialEq, Eq)]
struct Owner {
    uid: u32,
    logind_session: String,
}

pub struct Bridge {
    connection: Connection,
    owner: String,
    bus_id: String,
    owner_pid: u32,
    plans: HashMap<String, Owner>,
}

impl Bridge {
    pub fn connect() -> Result<Self, ErrorCode> {
        let connection = zbus::blocking::connection::Builder::address(
            "unix:path=/run/dbus/system_bus_socket",
        )
        .map_err(|_| ErrorCode::UnsupportedCapability)?
        .method_timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| ErrorCode::UnsupportedCapability)?;
        let (owner, bus_id, owner_pid) = subject(&connection)?;
        Ok(Self {
            connection,
            owner,
            bus_id,
            owner_pid,
            plans: HashMap::new(),
        })
    }

    pub fn prepare_package(
        &mut self,
        peer: &Peer,
        operation: &str,
        package_id: &str,
        text: &str,
    ) -> Result<Value, ErrorCode> {
        let owner = owner(peer)?;
        if !matches!(operation, "install" | "remove")
            || package_id.is_empty()
            || package_id.len() > 64
            || !package_id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            || text.trim().is_empty()
            || text.len() > 8192
            || text.contains('\0')
        {
            return Err(ErrorCode::InvalidArgument);
        }
        let request = json!({
            "schema_version": 1,
            "request_id": uuid::Uuid::new_v4().to_string(),
            "operation": "prepare",
            "mode": "act",
            "intent_text": text,
            "intent": {
                "action": if operation == "install" { "install_package" } else { "remove_package" },
                "package_id": package_id,
            },
        })
        .to_string();
        if request.len() > MAX_TASK_BYTES {
            return Err(ErrorCode::ResourceExhausted);
        }
        let value = self.call("Prepare", &(request.as_str(),))?;
        let id = value["data"]["plan_id"]
            .as_str()
            .filter(|id| crate::uuid(id))
            .ok_or(ErrorCode::TargetChanged)?
            .to_owned();
        if !value["data"]["plan_sha256"].as_str().is_some_and(digest) {
            return Err(ErrorCode::TargetChanged);
        }
        self.plans.insert(id, owner);
        Ok(value)
    }

    pub fn transaction(&mut self, peer: &Peer, id: &str) -> Result<Value, ErrorCode> {
        self.check(peer, id)?;
        self.call("GetTransaction", &(id,))
    }

    pub fn authorize(&mut self, peer: &Peer, id: &str) -> Result<Value, ErrorCode> {
        let hash = self.plan_hash(peer, id)?;
        self.call("Authorize", &(id, hash.as_str()))
    }

    pub fn execute(&mut self, peer: &Peer, id: &str) -> Result<Value, ErrorCode> {
        let hash = self.plan_hash(peer, id)?;
        self.call("Execute", &(id, hash.as_str()))
    }

    pub fn rollback_plan(&mut self, peer: &Peer, id: &str) -> Result<Value, ErrorCode> {
        self.check(peer, id)?;
        self.call("RequestRollback", &(id,))
    }

    fn plan_hash(&mut self, peer: &Peer, id: &str) -> Result<String, ErrorCode> {
        self.check(peer, id)?;
        self.call("GetPlan", &(id,))?["data"]["plan_sha256"]
            .as_str()
            .filter(|value| digest(value))
            .map(str::to_owned)
            .ok_or(ErrorCode::TargetChanged)
    }

    fn check(&self, peer: &Peer, id: &str) -> Result<(), ErrorCode> {
        if !crate::uuid(id) {
            return Err(ErrorCode::InvalidArgument);
        }
        if self.plans.get(id) != Some(&owner(peer)?) {
            return Err(ErrorCode::PermissionDenied);
        }
        Ok(())
    }

    fn call<B: serde::Serialize + zbus::zvariant::DynamicType>(
        &self,
        method: &str,
        body: &B,
    ) -> Result<Value, ErrorCode> {
        if subject(&self.connection)? != (self.owner.clone(), self.bus_id.clone(), self.owner_pid) {
            return Err(ErrorCode::TargetChanged);
        }
        let proxy = Proxy::new(&self.connection, self.owner.as_str(), PATH, INTERFACE)
            .map_err(|_| ErrorCode::UnsupportedCapability)?;
        let raw: String = proxy.call(method, body).map_err(client_error)?;
        if subject(&self.connection)? != (self.owner.clone(), self.bus_id.clone(), self.owner_pid) {
            return Err(ErrorCode::TargetChanged);
        }
        if raw.len() > MAX_FRAME_BYTES {
            return Err(ErrorCode::ResourceExhausted);
        }
        serde_json::from_str(&raw).map_err(|_| ErrorCode::InvalidArgument)
    }
}

fn owner(peer: &Peer) -> Result<Owner, ErrorCode> {
    Ok(Owner {
        uid: peer.uid,
        logind_session: peer
            .logind_session
            .clone()
            .ok_or(ErrorCode::AuthRequired)?,
    })
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn subject(connection: &Connection) -> Result<(String, String, u32), ErrorCode> {
    let bus = Proxy::new(
        connection,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .map_err(|_| ErrorCode::UnsupportedCapability)?;
    let owner: String = bus
        .call("GetNameOwner", &(NAME,))
        .map_err(|_| ErrorCode::UnsupportedCapability)?;
    if !owner.starts_with(':') {
        return Err(ErrorCode::PermissionDenied);
    }
    let credentials: zbus::fdo::ConnectionCredentials = bus
        .call("GetConnectionCredentials", &(owner.as_str(),))
        .map_err(|_| ErrorCode::PermissionDenied)?;
    if credentials.unix_user_id() != Some(0) {
        return Err(ErrorCode::PermissionDenied);
    }
    let pid = credentials.process_id().ok_or(ErrorCode::PermissionDenied)?;
    let bus_id: String = bus.call("GetId", &()).map_err(|_| ErrorCode::PermissionDenied)?;
    Ok((owner, bus_id, pid))
}

fn client_error(error: zbus::Error) -> ErrorCode {
    if let zbus::Error::MethodError(name, _, _) = &error {
        if let Some(code) = name.as_str().strip_prefix("org.aios.Error.") {
            return serde_json::from_value(Value::String(code.into()))
                .unwrap_or(ErrorCode::PartialResult);
        }
    }
    match error {
        zbus::Error::InputOutput(error)
            if error.kind() == std::io::ErrorKind::TimedOut =>
        {
            ErrorCode::DeadlineExceeded
        }
        _ => ErrorCode::PartialResult,
    }
}
