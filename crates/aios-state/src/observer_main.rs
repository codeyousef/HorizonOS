//! Fixed, unprivileged systemd/udev event observer.
//! It records only bounded counters and loss state; no event payload becomes authority.
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    time::{Duration, Instant},
};

const RUNTIME: &str = "/run/aios-observer";
const STATUS: &str = "/run/aios-observer/status.json";
const TEMP: &str = "/run/aios-observer/status.json.new";

#[derive(Eq, PartialEq, Serialize)]
struct Status {
    schema_version: u32,
    pid: u32,
    systemd_subscription: bool,
    device_subscription: bool,
    systemd_notifications: u64,
    device_notifications: u64,
    event_loss_observed: bool,
    mutation_authority: bool,
    network_egress: bool,
}

fn observer_uid() -> io::Result<u32> {
    let account = unsafe { libc::getpwnam(c"aios-observer".as_ptr()) };
    if account.is_null() {
        return Err(io::Error::other("observer account unavailable"));
    }
    let uid = unsafe { (*account).pw_uid };
    if uid == 0 || unsafe { libc::getuid() } != uid || unsafe { libc::geteuid() } != uid {
        return Err(io::Error::other("observer identity mismatch"));
    }
    Ok(uid)
}

fn runtime(uid: u32) -> io::Result<()> {
    let metadata = fs::symlink_metadata(RUNTIME)?;
    if !metadata.is_dir()
        || metadata.uid() != uid
        || metadata.mode() & 0o777 != 0o700
        || fs::canonicalize(RUNTIME)? != Path::new(RUNTIME)
    {
        return Err(io::Error::other("unsafe observer runtime directory"));
    }
    Ok(())
}

fn publish(uid: u32, status: &Status) -> io::Result<()> {
    runtime(uid)?;
    match fs::symlink_metadata(TEMP) {
        Ok(metadata) if metadata.is_file() && metadata.uid() == uid => fs::remove_file(TEMP)?,
        Ok(_) => return Err(io::Error::other("unsafe observer status temporary")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let bytes =
        serde_json::to_vec(status).map_err(|_| io::Error::other("status serialization failed"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(TEMP)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(TEMP, STATUS)?;
    let metadata = fs::symlink_metadata(STATUS)?;
    if !metadata.is_file() || metadata.uid() != uid || metadata.mode() & 0o777 != 0o600 {
        return Err(io::Error::other("unsafe observer status file"));
    }
    Ok(())
}

fn run() -> io::Result<()> {
    let uid = observer_uid()?;
    runtime(uid)?;
    let mut systemd = aios_system::services::events::SystemdEvents::connect().ok();
    let mut devices = aios_system::devices::DeviceEvents::connect().ok();
    let mut last_systemd_attempt = Instant::now();
    let mut last_device_attempt = Instant::now();
    let mut systemd_notifications = 0u64;
    let mut device_notifications = 0u64;
    let mut loss = systemd.is_none() || devices.is_none();
    let mut published = None;
    loop {
        if let Some(events) = systemd.as_mut() {
            match events.poll() {
                Ok(batch) => {
                    systemd_notifications =
                        systemd_notifications.saturating_add(batch.notifications);
                    loss |= batch.loss;
                    if batch.loss {
                        systemd = None;
                        last_systemd_attempt = Instant::now();
                    }
                }
                Err(_) => {
                    systemd = None;
                    loss = true;
                    last_systemd_attempt = Instant::now();
                }
            }
        } else if last_systemd_attempt.elapsed() >= Duration::from_secs(5) {
            last_systemd_attempt = Instant::now();
            systemd = aios_system::services::events::SystemdEvents::connect().ok();
        }
        if let Some(events) = devices.as_mut() {
            match events.poll() {
                Ok(batch) => {
                    device_notifications = device_notifications.saturating_add(batch.notifications);
                    loss |= batch.loss;
                    if batch.loss {
                        devices = None;
                        last_device_attempt = Instant::now();
                    }
                }
                Err(_) => {
                    devices = None;
                    loss = true;
                    last_device_attempt = Instant::now();
                }
            }
        } else if last_device_attempt.elapsed() >= Duration::from_secs(5) {
            last_device_attempt = Instant::now();
            devices = aios_system::devices::DeviceEvents::connect().ok();
        }
        let status = Status {
            schema_version: 1,
            pid: std::process::id(),
            systemd_subscription: systemd.is_some(),
            device_subscription: devices.is_some(),
            systemd_notifications,
            device_notifications,
            event_loss_observed: loss,
            mutation_authority: false,
            network_egress: false,
        };
        if published.as_ref() != Some(&status) {
            publish(uid, &status)?;
            published = Some(status);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("aios-observerd: refused: {error}");
        std::process::exit(1);
    }
}
