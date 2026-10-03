//! Deterministic broker preparation. No model shell, arbitrary Nix, or activation.
pub mod approval;
pub mod baseline;
pub mod bus;
pub mod caller;
pub mod candidate;
pub mod health;
pub mod ledger;
pub mod native;
use aios_protocol::contracts::canonical_json;
use serde::Serialize;
use sha2::{Digest, Sha256};
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Authority,
    TargetChanged,
    Ownership,
    Integrity,
    Io,
    Ledger,
    Conflict,
    NotFound,
    State,
    Expired,
    ResourcePermissionRequired,
    ActivationUnavailable,
    AuthRequired,
    Health,
}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}
impl From<rusqlite::Error> for Error {
    fn from(_: rusqlite::Error) -> Self {
        Self::Ledger
    }
}
impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        Self::Invalid
    }
}
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(crate) fn uuid(s: &str) -> bool {
    uuid::Uuid::parse_str(s).is_ok_and(|u| u.hyphenated().to_string() == s)
}
pub(crate) fn canonical<T: Serialize>(v: &T) -> Result<Vec<u8>> {
    canonical_json(&serde_json::to_value(v)?).map_err(|_| Error::Invalid)
}
pub(crate) fn store(s: &str) -> bool {
    let Some((hash, label)) = s
        .strip_prefix("/nix/store/")
        .and_then(|s| s.split_once('-'))
    else {
        return false;
    };
    hash.len() == 32
        && hash
            .bytes()
            .all(|b| b"0123456789abcdfghijklmnpqrsvwxyz".contains(&b))
        && !label.is_empty()
        && label.len() <= 192
        && label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
}
