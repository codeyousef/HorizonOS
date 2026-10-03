//! Native, separate intended/profile/running/boot observations. No client paths.
use crate::{
    Error, Result, candidate::InstalledTemplate, ledger::Baseline, native::VerifiedTarget, sha256,
    store,
};
use aios_state::Compiled;
use std::path::{Path, PathBuf};

fn system_root(path: PathBuf) -> Result<String> {
    let value = path.to_str().ok_or(Error::Integrity)?;
    if !store(value)
        || !value
            .split_once('-')
            .is_some_and(|(_, label)| label.starts_with("nixos-system-"))
    {
        return Err(Error::Integrity);
    }
    Ok(value.into())
}
fn text(bytes: &[u8]) -> Result<&str> {
    if bytes.len() > 65536
        || bytes
            .iter()
            .any(|b| *b == 0 || (*b < 32 && !b"\n\r\t".contains(b)))
    {
        return Err(Error::Integrity);
    }
    std::str::from_utf8(bytes).map_err(|_| Error::Integrity)
}
fn generation(name: &str) -> bool {
    name.strip_prefix("nixos-generation-")
        .and_then(|s| s.strip_suffix(".conf"))
        .is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 10
                && !s.starts_with('0')
                && s.bytes().all(|b| b.is_ascii_digit())
        })
}
fn loader_default(bytes: &[u8]) -> Result<String> {
    let entries: Vec<_> = text(bytes)?
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            (fields.next() == Some("default")).then(|| fields.collect::<Vec<_>>())
        })
        .collect();
    if entries.len() != 1 || entries[0].len() != 1 || !generation(entries[0][0]) {
        return Err(Error::Integrity);
    }
    Ok(entries[0][0].into())
}
fn boot_closure(bytes: &[u8]) -> Result<String> {
    let entries: Vec<_> = text(bytes)?
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            (fields.next() == Some("options")).then(|| fields.collect::<Vec<_>>())
        })
        .collect();
    if entries.len() != 1 {
        return Err(Error::Integrity);
    }
    let roots: Vec<_> = entries[0]
        .iter()
        .filter_map(|s| s.strip_prefix("init="))
        .collect();
    if roots.len() != 1 {
        return Err(Error::Integrity);
    }
    system_root(PathBuf::from(
        roots[0].strip_suffix("/init").ok_or(Error::Integrity)?,
    ))
}
fn payloads(bytes: &[u8]) -> Result<(String, String)> {
    let mut kernel = None;
    let mut initrd = None;
    for line in text(bytes)?.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        let slot = match fields.first().copied() {
            Some("linux") => &mut kernel,
            Some("initrd") => &mut initrd,
            _ => continue,
        };
        if fields.len() != 2 || slot.is_some() {
            return Err(Error::Integrity);
        }
        let file = fields[1]
            .strip_prefix("/EFI/nixos/")
            .ok_or(Error::Integrity)?;
        if file.is_empty()
            || file.len() > 256
            || matches!(file, "." | "..")
            || !file
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
        {
            return Err(Error::Integrity);
        }
        *slot = Some(fields[1].to_owned());
    }
    Ok((
        kernel.ok_or(Error::Integrity)?,
        initrd.ok_or(Error::Integrity)?,
    ))
}
fn firmware_entry(bytes: &[u8]) -> Result<String> {
    if bytes.len() < 6 || bytes.len() > 4096 || bytes.len() % 2 != 0 {
        return Err(Error::Integrity);
    }
    let words: Vec<_> = bytes[4..]
        .chunks_exact(2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .collect();
    if words.last() != Some(&0) || words[..words.len() - 1].contains(&0) {
        return Err(Error::Integrity);
    }
    String::from_utf16(&words[..words.len() - 1]).map_err(|_| Error::Integrity)
}

/// The system profile is the administrator's persisted intended declaration.
/// Its managed bytes are read independently of the current running closure.
/// Revisions must match the installed template; divergent pointers are recorded,
/// never silently equated. Unresolved firmware overrides fail closed.
pub struct NativeBaseline {
    target: VerifiedTarget,
    template_sha256: String,
    pub baseline: Baseline,
    pub managed: Compiled,
    pub intended_source: String,
}
impl NativeBaseline {
    pub fn capture(template: &InstalledTemplate) -> Result<Self> {
        let target = VerifiedTarget::enroll()?;
        let running = system_root(target.resolve_store_object(Path::new("/run/current-system"))?)?;
        let profile =
            system_root(target.resolve_store_object(Path::new("/nix/var/nix/profiles/system"))?)?;
        let loader = target.systemd_boot_file(None)?;
        let entry = loader_default(&loader)?;
        let entry_bytes = target.systemd_boot_file(Some(&entry))?;
        let boot = boot_closure(&entry_bytes)?;
        let once = target.firmware_boot_override(true)?;
        let default = target.firmware_boot_override(false)?;
        if once
            .as_deref()
            .map(firmware_entry)
            .transpose()?
            .is_some_and(|s| !s.is_empty())
            || default
                .as_deref()
                .map(firmware_entry)
                .transpose()?
                .is_some_and(|s| !s.is_empty() && s != entry)
        {
            return Err(Error::Conflict);
        }
        for closure in [&running, &profile, &boot] {
            target.verify_system_enrollment(closure)?;
        }
        let (kernel, initrd) = payloads(&entry_bytes)?;
        let kernel_hash = target.systemd_boot_payload_sha256(&kernel, 256 * 1024 * 1024)?;
        let initrd_hash = target.systemd_boot_payload_sha256(&initrd, 256 * 1024 * 1024)?;
        if kernel_hash
            != target
                .store_artifact(&Path::new(&boot).join("kernel"), 256 * 1024 * 1024, false)?
                .sha256
            || initrd_hash
                != target
                    .store_artifact(&Path::new(&boot).join("initrd"), 256 * 1024 * 1024, false)?
                    .sha256
        {
            return Err(Error::Integrity);
        }
        let intended_source = format!("{profile}/etc/aios/managed.json");
        let bytes = target.store_text(Path::new(&intended_source))?.into_bytes();
        let managed = template
            .catalog()
            .compile(&bytes)
            .map_err(|_| Error::TargetChanged)?;
        if bytes != managed.bytes {
            return Err(Error::Integrity);
        }
        let metadata = crate::canonical(&(
            entry,
            sha256(&loader),
            sha256(&entry_bytes),
            kernel_hash,
            initrd_hash,
            once,
            default,
        ))?;
        target.recheck()?;
        Ok(Self {
            target,
            template_sha256: template.digest().into(),
            intended_source,
            baseline: Baseline {
                intended_manifest_sha256: managed.digest.clone(),
                running_closure: running,
                profile_closure: profile,
                boot_selected_closure: boot,
                boot_metadata_sha256: sha256(&metadata),
            },
            managed,
        })
    }
    pub fn recheck(&self) -> Result<()> {
        self.target.recheck()?;
        let template = InstalledTemplate::from_installed()?;
        if template.digest() != self.template_sha256 {
            return Err(Error::TargetChanged);
        }
        let current = Self::capture(&template)?;
        if current.baseline != self.baseline
            || current.intended_source != self.intended_source
            || current.managed.bytes != self.managed.bytes
        {
            return Err(Error::TargetChanged);
        }
        self.target.recheck()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ROOT: &str = "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-nixos-system-fixture";
    #[test]
    fn fixed_boot_selection_has_no_wildcards_duplicate_or_traversal_fallback() {
        assert_eq!(
            loader_default(b"timeout 3\ndefault nixos-generation-12.conf\n").unwrap(),
            "nixos-generation-12.conf"
        );
        for value in [
            "",
            "default *",
            "default nixos-generation-01.conf",
            "default ../../entry",
            "default nixos-generation-1.conf extra",
            "default nixos-generation-1.conf\ndefault nixos-generation-2.conf",
        ] {
            assert!(loader_default(value.as_bytes()).is_err());
        }
        assert_eq!(
            boot_closure(format!("options quiet init={ROOT}/init\n").as_bytes()).unwrap(),
            ROOT
        );
        for value in [
            format!("options init={ROOT}/init init={ROOT}/init"),
            format!("options init={ROOT}/init\noptions init={ROOT}/init"),
            "options init=/tmp/evil/init".into(),
        ] {
            assert!(boot_closure(value.as_bytes()).is_err());
        }
        assert!(payloads(b"linux /EFI/nixos/k\ninitrd /EFI/nixos/i").is_ok());
        for value in [
            "linux /EFI/nixos/../evil\ninitrd /EFI/nixos/i",
            "linux /EFI/nixos/k\nlinux /EFI/nixos/k\ninitrd /EFI/nixos/i",
            "linux /EFI/nixos/k",
        ] {
            assert!(payloads(value.as_bytes()).is_err());
        }
    }
    #[test]
    fn firmware_selection_is_bounded_terminated_utf16() {
        let mut bytes = vec![7, 0, 0, 0];
        for word in "nixos-generation-2.conf\0".encode_utf16() {
            bytes.extend(word.to_le_bytes());
        }
        assert_eq!(firmware_entry(&bytes).unwrap(), "nixos-generation-2.conf");
        assert!(firmware_entry(&bytes[..bytes.len() - 2]).is_err());
        assert!(firmware_entry(&[0; 4098]).is_err());
    }
}
