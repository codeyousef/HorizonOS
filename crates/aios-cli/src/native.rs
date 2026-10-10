use aios_protocol::contracts::ErrorCode;
use serde_json::{Value, json};
use std::time::Duration;
use zbus::blocking::{Connection, Proxy};

const SYSTEM_NAME: &str = "org.aios.System1";
const EXECUTOR_NAME: &str = "org.aios.Executor1";
const SYSTEM_PATH: &str = "/org/aios/System1";
const PACKAGES_PATH: &str = "/org/aios/Packages1";
const EXECUTOR_PATH: &str = "/org/aios/Executor1";

pub struct Client {
    connection: Connection,
    owner: String,
    bus_id: String,
    owner_pid: u32,
}

impl Client {
    pub fn connect() -> Result<Self, ErrorCode> {
        let connection = zbus::blocking::connection::Builder::address("unix:path=/run/dbus/system_bus_socket")
            .map_err(|_| ErrorCode::UnsupportedCapability)?
            .method_timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| ErrorCode::UnsupportedCapability)?;
        let (owner, bus_id, owner_pid) = subject(&connection, SYSTEM_NAME)?;
        Ok(Self { connection, owner, bus_id, owner_pid })
    }

    fn verify(&self) -> Result<(), ErrorCode> {
        let current = subject(&self.connection, SYSTEM_NAME)?;
        if current != (self.owner.clone(), self.bus_id.clone(), self.owner_pid) {
            return Err(ErrorCode::TargetChanged);
        }
        Ok(())
    }

    fn call<B: serde::Serialize + zbus::zvariant::DynamicType>(
        &self,
        path: &str,
        interface: &str,
        method: &str,
        body: &B,
    ) -> Result<Value, ErrorCode> {
        self.verify()?;
        let proxy = Proxy::new(&self.connection, self.owner.as_str(), path, interface)
            .map_err(|_| ErrorCode::UnsupportedCapability)?;
        let raw: String = proxy.call(method, body).map_err(client_error)?;
        self.verify()?;
        if raw.len() > aios_protocol::MAX_FRAME_BYTES {
            return Err(ErrorCode::ResourceExhausted);
        }
        serde_json::from_str(&raw).map_err(|_| ErrorCode::InvalidArgument)
    }

    pub fn package_search(&self, query: &str) -> Result<Value, ErrorCode> {
        self.package_action("Search", "packages.search", json!({"query":query}))
    }

    pub fn package_info(&self, package_id: &str) -> Result<Value, ErrorCode> {
        self.package_action("Info", "packages.info", json!({"package_id":package_id}))
    }

    fn package_action(&self, method: &str, action: &str, arguments: Value) -> Result<Value, ErrorCode> {
        let request = json!({"schema_version":1,"request_id":uuid::Uuid::new_v4().to_string(),
            "operation":{"kind":"invoke","tool_call":{"kind":"tool_call","action_id":action,"arguments":arguments}}}).to_string();
        if request.len() > aios_protocol::MAX_TASK_BYTES {
            return Err(ErrorCode::ResourceExhausted);
        }
        let value = self.call(PACKAGES_PATH, "org.aios.Packages1", method, &(request.as_str(),))?;
        aios_protocol::validation::validate_result(action, &serde_json::to_vec(&value).map_err(|_| ErrorCode::InvalidArgument)?)?;
        Ok(value)
    }

    pub fn graph_status(&self) -> Result<Value, ErrorCode> {
        let value = self.call(SYSTEM_PATH, "org.aios.System1", "GraphStatus", &())?;
        if value["schema_version"] != 1 || value["scope"] != "system" || value["model_invoked"] != false
            || value["execution_authority"] != false || !value["boot"].is_string() {
            return Err(ErrorCode::TargetChanged);
        }
        Ok(value)
    }
}


pub struct ExecutorClient {
    connection: Connection,
    owner: String,
    bus_id: String,
    owner_pid: u32,
}

impl ExecutorClient {
    pub fn connect() -> Result<Self, ErrorCode> {
        let connection = zbus::blocking::connection::Builder::address(
            "unix:path=/run/dbus/system_bus_socket",
        )
        .map_err(|_| ErrorCode::UnsupportedCapability)?
        .method_timeout(Duration::from_secs(150))
        .build()
        .map_err(|_| ErrorCode::UnsupportedCapability)?;
        let (owner, bus_id, owner_pid) = subject(&connection, EXECUTOR_NAME)?;
        Ok(Self { connection, owner, bus_id, owner_pid })
    }

    fn call<B: serde::Serialize + zbus::zvariant::DynamicType>(
        &self,
        method: &str,
        body: &B,
    ) -> Result<Value, ErrorCode> {
        if subject(&self.connection, EXECUTOR_NAME)?
            != (self.owner.clone(), self.bus_id.clone(), self.owner_pid)
        {
            return Err(ErrorCode::TargetChanged);
        }
        let proxy = Proxy::new(
            &self.connection,
            self.owner.as_str(),
            EXECUTOR_PATH,
            "org.aios.Executor1",
        )
        .map_err(|_| ErrorCode::UnsupportedCapability)?;
        let raw: String = proxy.call(method, body).map_err(client_error)?;
        if subject(&self.connection, EXECUTOR_NAME)?
            != (self.owner.clone(), self.bus_id.clone(), self.owner_pid)
        {
            return Err(ErrorCode::TargetChanged);
        }
        if raw.len() > aios_protocol::MAX_FRAME_BYTES {
            return Err(ErrorCode::ResourceExhausted);
        }
        serde_json::from_str(&raw).map_err(|_| ErrorCode::InvalidArgument)
    }

    pub fn prepare_package(
        &self,
        operation: &str,
        package_id: &str,
        text: &str,
    ) -> Result<Value, ErrorCode> {
        if !matches!(operation, "install" | "remove") {
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
        if request.len() > aios_protocol::MAX_TASK_BYTES {
            return Err(ErrorCode::ResourceExhausted);
        }
        self.call("Prepare", &(request.as_str(),))
    }

    pub fn plan(&self, id: &str) -> Result<Value, ErrorCode> {
        self.call("GetPlan", &(id,))
    }

    pub fn transaction(&self, id: &str) -> Result<Value, ErrorCode> {
        self.call("GetTransaction", &(id,))
    }

    pub fn cancel(&self, id: &str) -> Result<Value, ErrorCode> {
        self.call("Cancel", &(id,))
    }

    pub fn authorize(&self, id: &str, hash: &str) -> Result<Value, ErrorCode> {
        self.call("Authorize", &(id, hash))
    }

    pub fn execute(&self, id: &str, hash: &str) -> Result<Value, ErrorCode> {
        self.call("Execute", &(id, hash))
    }

    pub fn rollback_plan(&self, id: &str) -> Result<Value, ErrorCode> {
        self.call("RequestRollback", &(id,))
    }
}

fn subject(connection: &Connection, name: &str) -> Result<(String, String, u32), ErrorCode> {
    let bus = Proxy::new(connection, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus")
        .map_err(|_| ErrorCode::UnsupportedCapability)?;
    let owner: String = bus.call("GetNameOwner", &(name,)).map_err(|_| ErrorCode::UnsupportedCapability)?;
    if !owner.starts_with(':') {
        return Err(ErrorCode::PermissionDenied);
    }
    let credentials: zbus::fdo::ConnectionCredentials = bus.call("GetConnectionCredentials", &(owner.as_str(),))
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
            return serde_json::from_value(Value::String(code.into())).unwrap_or(ErrorCode::PartialResult);
        }
    }
    match error {
        zbus::Error::InputOutput(error) if error.kind() == std::io::ErrorKind::TimedOut => ErrorCode::DeadlineExceeded,
        _ => ErrorCode::PartialResult,
    }
}
