//! Native read-only intake. No plan input confers activation authority.
use crate::{Candidate, Closure, Error, Identity, Plan, Result, hash, valid_store};
use aios_exec::native::{StoreArtifact, VerifiedTarget};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize)]
pub struct BootSelection {
    pub entry: String,
    pub closure: String,
    pub loader_sha256: String,
    pub entry_sha256: String,
    pub kernel_sha256: String,
    pub initrd_sha256: String,
    pub firmware_default_present: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct IntakeEvidence {
    pub identity: Identity,
    pub running: Closure,
    pub profile: Closure,
    pub boot: Closure,
    pub boot_selection: BootSelection,
    pub guard: StoreArtifact,
    pub nix_env: StoreArtifact,
    pub systemctl: StoreArtifact,
    pub boot_time_ms: u64,
    pub health: aios_exec::health::HealthEvidence,
}
/// This cannot be deserialized or constructed from client observations.
pub struct NativeIntake {
    target: VerifiedTarget,
    evidence: IntakeEvidence,
}
fn native<T>(value: aios_exec::Result<T>) -> Result<T> {
    value.map_err(|error| match error {
        aios_exec::Error::Authority => Error::Authority,
        aios_exec::Error::TargetChanged => Error::TargetMismatch,
        aios_exec::Error::Health => Error::Health,
        _ => Error::Integrity,
    })
}
pub fn boot_time_ms() -> Result<u64> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut time) } != 0
        || time.tv_sec < 0
        || !(0..1_000_000_000).contains(&time.tv_nsec)
    {
        return Err(Error::Integrity);
    }
    (time.tv_sec as u64)
        .checked_mul(1000)
        .and_then(|ms| ms.checked_add(time.tv_nsec as u64 / 1_000_000))
        .ok_or(Error::Integrity)
}
fn system_root(value: PathBuf) -> Result<String> {
    let path = value.to_str().ok_or(Error::Integrity)?;
    if !valid_store(path)
        || !path
            .split_once('-')
            .is_some_and(|(_, label)| label.starts_with("nixos-system-"))
    {
        return Err(Error::Integrity);
    }
    Ok(path.into())
}
fn text(bytes: &[u8]) -> Result<&str> {
    if bytes.len() > 65536
        || bytes
            .iter()
            .any(|b| *b == 0 || (*b < 32 && !b"\n\r\t".contains(b)))
    {
        return Err(Error::BootSelection);
    }
    std::str::from_utf8(bytes).map_err(|_| Error::BootSelection)
}
fn generation_entry(value: &str) -> bool {
    value
        .strip_prefix("nixos-generation-")
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
            if fields.next() == Some("default") {
                Some(fields.collect::<Vec<_>>())
            } else {
                None
            }
        })
        .collect();
    if entries.len() != 1 || entries[0].len() != 1 || !generation_entry(entries[0][0]) {
        return Err(Error::BootSelection);
    }
    Ok(entries[0][0].into())
}
fn boot_closure(bytes: &[u8]) -> Result<String> {
    let options: Vec<_> = text(bytes)?
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            if fields.next() == Some("options") {
                Some(fields.collect::<Vec<_>>())
            } else {
                None
            }
        })
        .collect();
    if options.len() != 1 {
        return Err(Error::BootSelection);
    }
    let inits: Vec<_> = options[0]
        .iter()
        .filter_map(|s| s.strip_prefix("init="))
        .collect();
    if inits.len() != 1 {
        return Err(Error::BootSelection);
    }
    let root = inits[0].strip_suffix("/init").ok_or(Error::BootSelection)?;
    system_root(PathBuf::from(root)).map_err(|_| Error::BootSelection)
}
fn boot_payloads(bytes: &[u8]) -> Result<(String, String)> {
    let mut kernel = None;
    let mut initrd = None;
    for line in text(bytes)?.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        let selected = match fields.first().copied() {
            Some("linux") => &mut kernel,
            Some("initrd") => &mut initrd,
            _ => continue,
        };
        if fields.len() != 2 || selected.is_some() {
            return Err(Error::BootSelection);
        }
        let name = fields[1]
            .strip_prefix("/EFI/nixos/")
            .ok_or(Error::BootSelection)?;
        if name.is_empty()
            || name.len() > 256
            || matches!(name, "." | "..")
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
        {
            return Err(Error::BootSelection);
        }
        *selected = Some(fields[1].to_owned());
    }
    Ok((
        kernel.ok_or(Error::BootSelection)?,
        initrd.ok_or(Error::BootSelection)?,
    ))
}
fn firmware_entry(bytes: &[u8]) -> Result<String> {
    // efivarfs prefixes UTF-16LE data with four attribute bytes.
    if bytes.len() < 6 || bytes.len() > 4096 || bytes.len() % 2 != 0 {
        return Err(Error::BootSelection);
    }
    let words: Vec<_> = bytes[4..]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    if words.last() != Some(&0) || words[..words.len() - 1].contains(&0) {
        return Err(Error::BootSelection);
    }
    String::from_utf16(&words[..words.len() - 1]).map_err(|_| Error::BootSelection)
}
fn boot_adapter(wrapper: &str) -> Result<String> {
    let entries: Vec<_> = wrapper
        .lines()
        .filter_map(|line| line.trim().strip_prefix("export INSTALL_BOOTLOADER="))
        .collect();
    if entries.len() != 1 {
        return Err(Error::Integrity);
    }
    let value = entries[0];
    let path = value
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .or_else(|| value.strip_prefix('"').and_then(|s| s.strip_suffix('"')))
        .unwrap_or(value);
    if !valid_store(path) {
        return Err(Error::Integrity);
    }
    Ok(path.into())
}
fn closure(target: &VerifiedTarget, root: &str) -> Result<Closure> {
    if !valid_store(root) {
        return Err(Error::Integrity);
    }
    native(target.verify_system_enrollment(root))?;
    let activation = Path::new(root).join("bin/switch-to-configuration");
    let wrapper = native(target.store_text(&activation))?;
    let adapter = boot_adapter(&wrapper)?;
    Ok(Closure {
        path: root.into(),
        activation_sha256: native(target.store_artifact(&activation, 65536, true))?.sha256,
        kernel_sha256: native(target.store_artifact(
            &Path::new(root).join("kernel"),
            256 * 1024 * 1024,
            false,
        ))?
        .sha256,
        initrd_sha256: native(target.store_artifact(
            &Path::new(root).join("initrd"),
            256 * 1024 * 1024,
            false,
        ))?
        .sha256,
        boot_adapter_sha256: native(target.store_artifact(Path::new(&adapter), 1024 * 1024, true))?
            .sha256,
    })
}
impl NativeIntake {
    pub fn capture() -> Result<Self> {
        let target = native(VerifiedTarget::enroll())?;
        let observed = target.target();
        let identity = Identity {
            installation_uuid: observed.installation_uuid.clone(),
            dmi_uuid: observed.dmi_uuid.clone(),
            machine_id: observed.machine_id.clone(),
            boot_id: observed.boot_id.clone(),
            role: observed.role.clone(),
            disk_serial: observed.disk_serial.clone(),
            management_channel: observed.management_channel.clone(),
        };
        let running_path = system_root(native(
            target.resolve_store_object(Path::new("/run/current-system")),
        )?)?;
        let profile_path = system_root(native(
            target.resolve_store_object(Path::new("/nix/var/nix/profiles/system")),
        )?)?;
        let loader = native(target.systemd_boot_file(None))?;
        let entry = loader_default(&loader)?;
        let entry_bytes = native(target.systemd_boot_file(Some(&entry)))?;
        let boot_path = boot_closure(&entry_bytes)?;
        let once = native(target.firmware_boot_override(true))?;
        if once
            .as_deref()
            .map(firmware_entry)
            .transpose()?
            .is_some_and(|s| !s.is_empty())
        {
            return Err(Error::BootSelection);
        }
        let firmware = native(target.firmware_boot_override(false))?;
        if firmware
            .as_deref()
            .map(firmware_entry)
            .transpose()?
            .is_some_and(|s| !s.is_empty() && s != entry)
        {
            return Err(Error::BootSelection);
        }
        let running = closure(&target, &running_path)?;
        let profile = if profile_path == running.path {
            running.clone()
        } else {
            closure(&target, &profile_path)?
        };
        let boot = if boot_path == running.path {
            running.clone()
        } else if boot_path == profile.path {
            profile.clone()
        } else {
            closure(&target, &boot_path)?
        };
        let (kernel_payload, initrd_payload) = boot_payloads(&entry_bytes)?;
        let boot_kernel =
            native(target.systemd_boot_payload_sha256(&kernel_payload, 256 * 1024 * 1024))?;
        let boot_initrd =
            native(target.systemd_boot_payload_sha256(&initrd_payload, 256 * 1024 * 1024))?;
        if boot_kernel != boot.kernel_sha256 || boot_initrd != boot.initrd_sha256 {
            return Err(Error::BootSelection);
        }
        let installed_guard = Path::new("/run/current-system/sw/bin/aios-guard");
        let guard = native(target.store_artifact(installed_guard, 64 * 1024 * 1024, true))?;
        let myself = std::env::current_exe().map_err(|_| Error::Integrity)?;
        let current = native(target.store_artifact(&myself, 64 * 1024 * 1024, true))?;
        if guard.path != current.path || guard.sha256 != current.sha256 {
            return Err(Error::Integrity);
        }
        // The fixed self executable alias is inspected only to match its inode;
        // it is never admitted as a general procfs configuration path.
        use std::os::unix::fs::MetadataExt;
        let live = fs::metadata("/proc/self/exe").map_err(|_| Error::Integrity)?;
        let stored = fs::metadata(&guard.path).map_err(|_| Error::Integrity)?;
        if (live.dev(), live.ino()) != (stored.dev(), stored.ino()) {
            return Err(Error::Integrity);
        }
        let nix_env = native(target.store_artifact(
            Path::new("/run/current-system/sw/bin/nix-env"),
            64 * 1024 * 1024,
            true,
        ))?;
        let systemctl = native(target.store_artifact(
            Path::new("/run/current-system/sw/bin/systemctl"),
            64 * 1024 * 1024,
            true,
        ))?;
        let health = native(aios_exec::health::NativeHealth::capture())?;
        if &health.evidence().target != target.target() {
            return Err(Error::TargetMismatch);
        }
        native(target.recheck())?;
        let evidence = IntakeEvidence {
            identity,
            running,
            profile,
            boot,
            boot_selection: BootSelection {
                entry,
                closure: boot_path,
                loader_sha256: hash(&loader),
                entry_sha256: hash(&entry_bytes),
                kernel_sha256: boot_kernel,
                initrd_sha256: boot_initrd,
                firmware_default_present: firmware.is_some(),
            },
            guard,
            nix_env,
            systemctl,
            boot_time_ms: boot_time_ms()?,
            health: health.evidence().clone(),
        };
        Ok(Self { target, evidence })
    }
    pub fn evidence(&self) -> &IntakeEvidence {
        &self.evidence
    }
    /// Artifact binding is separate from root broker/developer authorization and
    /// build provenance. This method cannot arm, execute or register a plan.
    pub fn verify_plan_artifacts(&self, plan: &Plan) -> Result<()> {
        plan.validate()?;
        native(self.target.recheck())?;
        if plan.identity != self.evidence.identity
            || plan.prior.running != self.evidence.running
            || plan.prior.profile != self.evidence.profile
            || plan.prior.boot != self.evidence.boot
            || plan.retained_guard_sha256 != self.evidence.guard.sha256
        {
            return Err(Error::TargetMismatch);
        }
        match &plan.candidate {
            Candidate::System { closure: expected } => {
                if closure(&self.target, &expected.path)? != *expected {
                    return Err(Error::Integrity);
                }
            }
            Candidate::ModelOnly { .. } => return Err(Error::Adapter),
        }
        native(self.target.recheck())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loader_requires_one_exact_generation() {
        assert_eq!(
            loader_default(b"timeout 5\ndefault nixos-generation-7.conf\n").unwrap(),
            "nixos-generation-7.conf"
        );
        for input in [
            "",
            "default nixos-*",
            "default ../entry.conf",
            "default nixos-generation-0.conf",
            "default nixos-generation-01.conf",
            "default nixos-generation-7.conf extra",
            "default nixos-generation-7.conf\ndefault nixos-generation-8.conf",
            "default nixos-other-generation-7.conf",
        ] {
            assert!(loader_default(input.as_bytes()).is_err(), "{input}");
        }
    }
    #[test]
    fn selected_entry_requires_one_immutable_system_init() {
        let root = format!("/nix/store/{}-nixos-system-aios-dev-26.05", "a".repeat(32));
        assert_eq!(
            boot_closure(format!("title NixOS\noptions init={root}/init quiet\n").as_bytes())
                .unwrap(),
            root
        );
        for input in [
            format!("options init={root}/init init={root}/init"),
            format!("options init={root}/init\noptions quiet"),
            "options init=/etc/nixos/init".into(),
            "options quiet".into(),
            "options init=/nix/store/../init".into(),
            "options init=/nix/store/x\0/init".into(),
        ] {
            assert!(boot_closure(input.as_bytes()).is_err());
        }
    }
    #[test]
    fn firmware_decode_does_not_truncate_embedded_nuls_or_bad_utf16() {
        let mut valid = vec![7, 0, 0, 0];
        for word in "nixos-generation-7.conf".encode_utf16().chain([0]) {
            valid.extend(word.to_le_bytes());
        }
        assert_eq!(firmware_entry(&valid).unwrap(), "nixos-generation-7.conf");
        for input in [
            vec![],
            vec![7, 0, 0, 0, 1],
            vec![7, 0, 0, 0, 0, 0, 0, 0],
            vec![7, 0, 0, 0, 0, 0xd8, 0, 0],
        ] {
            assert!(firmware_entry(&input).is_err());
        }
    }
    #[test]
    fn boot_payloads_reject_duplicates_paths_and_missing_images() {
        assert_eq!(
            boot_payloads(b"linux /EFI/nixos/kernel.efi\ninitrd /EFI/nixos/initrd.efi").unwrap(),
            (
                "/EFI/nixos/kernel.efi".into(),
                "/EFI/nixos/initrd.efi".into()
            )
        );
        for input in [
            "linux /EFI/nixos/a\ninitrd /etc/x",
            "linux /EFI/nixos/../x\ninitrd /EFI/nixos/b",
            "linux /EFI/nixos/a\nlinux /EFI/nixos/b\ninitrd /EFI/nixos/c",
            "linux /EFI/nixos/a",
            "linux /EFI/nixos/a extra\ninitrd /EFI/nixos/c",
        ] {
            assert!(boot_payloads(input.as_bytes()).is_err());
        }
    }
    #[test]
    fn boot_adapter_is_one_literal_store_program_never_a_shell_expression() {
        let path = format!("/nix/store/{}-install-bootloader.sh", "a".repeat(32));
        assert_eq!(
            boot_adapter(&format!("export INSTALL_BOOTLOADER='{path}'\n")).unwrap(),
            path
        );
        for input in [
            format!("export INSTALL_BOOTLOADER='{path}'\nexport INSTALL_BOOTLOADER='{path}'"),
            "export INSTALL_BOOTLOADER='$(false)'".into(),
            "export INSTALL_BOOTLOADER='/etc/boot.sh'".into(),
            format!("export INSTALL_BOOTLOADER='{path} --force'"),
            "".into(),
        ] {
            assert!(boot_adapter(&input).is_err());
        }
    }
    #[test]
    fn native_intake_never_mints_root_authority_for_nonroot_tests() {
        if unsafe { libc::getuid() } != 0 {
            assert!(matches!(NativeIntake::capture(), Err(Error::Authority)));
        }
    }
    #[test]
    fn actual_boot_clock_is_monotonic() {
        let before = boot_time_ms().unwrap();
        assert!(boot_time_ms().unwrap() >= before);
    }
}
