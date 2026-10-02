use aios_session::{SharedState, State, serve_connection};
use std::{io, os::unix::{fs::{DirBuilderExt, MetadataExt, PermissionsExt}, net::UnixListener}, path::PathBuf,
    sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}}, thread};

fn run() -> io::Result<()> {
    if nix::unistd::geteuid().as_raw() == 0 {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied,"session broker requires a non-root user"));
    }
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        let runtime = PathBuf::from(format!("/run/user/{}", nix::unistd::geteuid()));
        let info = std::fs::symlink_metadata(&runtime)?;
        if std::fs::canonicalize(&runtime)? != runtime || !info.is_dir() || info.uid() != nix::unistd::geteuid().as_raw() || info.mode() & 0o077 != 0 {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied,"user runtime directory unavailable"));
        }
        let directory = runtime.join("aios");
        if !directory.exists() { std::fs::DirBuilder::new().mode(0o700).create(&directory)?; }
    }
    let path = match args.as_slice() {
        [] => PathBuf::from(format!("/run/user/{}/aios/session.sock", nix::unistd::geteuid())),
        [flag, path] if flag == "--socket" => PathBuf::from(path),
        _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "usage: aios-sessiond [--socket PRIVATE_PATH]")),
    };
    if !path.is_absolute() { return Err(io::Error::new(io::ErrorKind::InvalidInput,"socket must be absolute")); }
    let parent = path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput,"socket parent missing"))?;
    // Never create or chmod arbitrary caller paths, follow symlink directories,
    // replace an existing endpoint, or attach to another user's runtime directory.
    if std::fs::canonicalize(parent)? != parent { return Err(io::Error::new(io::ErrorKind::PermissionDenied,"socket parent contains symlink")); }
    let info = std::fs::symlink_metadata(parent)?;
    if !info.is_dir() || info.uid() != nix::unistd::geteuid().as_raw() || info.mode() & 0o077 != 0 {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied,"socket parent must be private and owned"));
    }
    let state: SharedState = Arc::new(Mutex::new(State::default()));
    // Default user deployment owns the public control name and the private
    // socket together. Isolated --socket fixtures export only the private API.
    let _bus = if args.is_empty() {
        Some(aios_session::bus::export_user_bus(state.clone()).map_err(io::Error::other)?)
    } else { None };
    if let Some(bus) = &_bus {
        let connection = bus.clone();
        let original = connection.unique_name().ok_or_else(|| io::Error::other("missing bus owner"))?.to_string();
        thread::spawn(move || loop {
            let owner = zbus::blocking::Proxy::new(&connection, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus")
                .and_then(|proxy| proxy.call::<_,_,String>("GetNameOwner", &(aios_session::bus::NAME,)));
            if !matches!(owner, Ok(ref name) if name == &original) {
                eprintln!("aios-sessiond: public bus owner unavailable");
                std::process::exit(1);
            }
            thread::sleep(std::time::Duration::from_secs(1));
        });
    }
    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    let active = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        let stream = stream?;
        if active.fetch_add(1, Ordering::AcqRel) >= 16 { active.fetch_sub(1, Ordering::AcqRel); continue; }
        let state = state.clone(); let active = active.clone();
        thread::spawn(move || {
            let _ = serve_connection(stream, state);
            active.fetch_sub(1, Ordering::AcqRel);
        });
    }
    Ok(())
}
fn main() { if let Err(error) = run() { eprintln!("aios-sessiond: {error}"); std::process::exit(1); } }
