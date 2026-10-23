//! Fixed socket-activated worker, never a path/shell execution interface.
use std::{io,os::fd::FromRawFd,os::unix::net::UnixStream};
fn run()->io::Result<()> {
    if std::env::args().skip(1).collect::<Vec<_>>()!=["--socket-worker"] || nix::unistd::geteuid().is_root() {
        return Err(io::Error::other("extractor requires its unprivileged socket unit"));
    }
    // StandardInput=socket passes exactly this accepted connection. Receiving
    // the source FD is a separate bounded SCM_RIGHTS operation on it.
    aios_files::extract::transport::serve_once(unsafe {UnixStream::from_raw_fd(0)})
}
fn main(){if let Err(error)=run(){eprintln!("aios-extract: {error}");std::process::exit(1);}}
