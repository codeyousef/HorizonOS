//! Reviewed desired-state compilation. No evaluation, execution, or authorization.
//! The caller supplies the installed catalog, authenticated grants and observations;
//! these are separate from model intent and cannot be deserialized as an approval.
use aios_protocol::contracts::canonical_json;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const MAX_MANIFEST_BYTES: usize = 65536;
pub const MAX_CATALOG_BYTES: usize = 262144;
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    RevisionChanged,
    UnknownPackage,
    WrongCapability,
    ProtectedTransport,
    UnfreeAcknowledgementRequired,
    DataReviewRequired,
    CatalogInvalid,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    canonical_json(&serde_json::to_value(value).map_err(|_| Error::InvalidInput)?)
        .map_err(|_| Error::InvalidInput)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CatalogEntry {
    pub id: String,
    pub attribute: Vec<String>,
    pub display_name: String,
    pub version: String,
    pub licenses: Vec<String>,
    pub unfree: bool,
    pub platform: String,
    pub binaries: Vec<String>,
    pub desktop_ids: Vec<String>,
    pub capability: Capability,
    pub metadata_revision: String,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    DesktopApplication,
    Postgresql17,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CatalogContent {
    pub schema_version: u32,
    pub base_template_revision: String,
    pub lock_sha256: String,
    pub nixpkgs_revision: String,
    pub installation_state_version: String,
    pub platform: String,
    pub packages: Vec<CatalogEntry>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CatalogEnvelope {
    catalog_revision: String,
    content: CatalogContent,
}
/// Construct only from installed administrator-reviewed catalog bytes. A digest
/// binds contents, but is not a signature or proof of trusted filesystem origin.
#[derive(Clone, Debug)]
pub struct Catalog {
    revision: String,
    content: CatalogContent,
}
impl Catalog {
    pub fn from_installed(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > MAX_CATALOG_BYTES {
            return Err(Error::CatalogInvalid);
        }
        let e: CatalogEnvelope =
            serde_json::from_slice(bytes).map_err(|_| Error::CatalogInvalid)?;
        let c = &e.content;
        let safe_label =
            |s: &str| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control);
        if !digest(&e.catalog_revision)
            || hash(&canonical(c)?) != e.catalog_revision
            || c.schema_version != 1
            || !digest(&c.base_template_revision)
            || !digest(&c.lock_sha256)
            || c.nixpkgs_revision.len() != 40
            || !c
                .nixpkgs_revision
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || c.installation_state_version != "26.05"
            || c.platform != "x86_64-linux"
            || c.packages.is_empty()
            || c.packages.len() > 256
            || !c.packages.windows(2).all(|p| p[0].id < p[1].id)
            || !c
                .packages
                .iter()
                .any(|p| p.id == "postgresql-17" && p.capability == Capability::Postgresql17)
        {
            return Err(Error::CatalogInvalid);
        }
        for p in &c.packages {
            if !id(&p.id)
                || p.attribute.is_empty()
                || p.attribute.len() > 4
                || !p.attribute.iter().all(|s| {
                    id(s)
                        || (s.len() <= 64
                            && !s.is_empty()
                            && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
                })
                || !safe_label(&p.display_name)
                || !safe_label(&p.version)
                || p.platform != c.platform
                || p.licenses.is_empty()
                || p.licenses.len() > 16
                || !p.licenses.iter().all(|s| safe_label(s))
                || !digest(&p.metadata_revision)
                || p.binaries.len() > 16
                || p.desktop_ids.len() > 16
                || !p.binaries.iter().chain(p.desktop_ids.iter()).all(|s| {
                    !s.is_empty()
                        && s.len() <= 128
                        && s.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                })
                || (p.capability == Capability::DesktopApplication
                    && p.binaries.is_empty()
                    && p.desktop_ids.is_empty())
                || (p.capability == Capability::Postgresql17
                    && (p.id != "postgresql-17"
                        || !p.version.starts_with("17.")
                        || !p.binaries.iter().any(|b| b == "pg_isready")))
            {
                return Err(Error::CatalogInvalid);
            }
            let mut metadata = serde_json::to_value(p).map_err(|_| Error::CatalogInvalid)?;
            metadata
                .as_object_mut()
                .ok_or(Error::CatalogInvalid)?
                .remove("metadata_revision");
            if hash(&canonical_json(&metadata).map_err(|_| Error::CatalogInvalid)?)
                != p.metadata_revision
            {
                return Err(Error::CatalogInvalid);
            }
        }
        Ok(Self {
            revision: e.catalog_revision,
            content: e.content,
        })
    }
    pub fn revision(&self) -> &str {
        &self.revision
    }
    pub fn content(&self) -> &CatalogContent {
        &self.content
    }
    pub fn entry(&self, requested: &str) -> Result<&CatalogEntry> {
        self.content
            .packages
            .iter()
            .find(|p| p.id == requested)
            .ok_or(Error::UnknownPackage)
    }
    pub fn defaults(&self) -> ManagedState {
        ManagedState {
            schema_version: 1,
            base_template_revision: self.content.base_template_revision.clone(),
            catalog_revision: self.revision.clone(),
            system_packages: vec![],
            services: Services::default(),
            power_policy: PowerPolicy::default(),
        }
    }
    pub fn compile(&self, bytes: &[u8]) -> Result<Compiled> {
        if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
            return Err(Error::InvalidInput);
        }
        let state: ManagedState = serde_json::from_slice(bytes).map_err(|_| Error::InvalidInput)?;
        self.compile_state(state)
    }
    /// Offline preliminary preview. The supplied baseline is not an observation
    /// of the installed machine, and the request carries no permission or data
    /// inspection evidence. The broker must prepare its own authoritative plan.
    pub fn preview_request(&self, bytes: &[u8]) -> Result<Preview> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Request {
            managed: Box<serde_json::value::RawValue>,
            intent: Intent,
        }
        if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
            return Err(Error::InvalidInput);
        }
        let request: Request = serde_json::from_slice(bytes).map_err(|_| Error::InvalidInput)?;
        self.prepare(
            &self.compile(request.managed.get().as_bytes())?,
            request.intent,
            DatabaseData::Unknown,
            &PreparationGrants::default(),
        )
    }
    fn compile_state(&self, mut state: ManagedState) -> Result<Compiled> {
        if state.schema_version != 1 {
            return Err(Error::InvalidInput);
        }
        if state.base_template_revision != self.content.base_template_revision
            || state.catalog_revision != self.revision
        {
            return Err(Error::RevisionChanged);
        }
        if state.system_packages.len() > 128 {
            return Err(Error::InvalidInput);
        }
        let mut seen = BTreeSet::new();
        for name in &state.system_packages {
            if !seen.insert(name.clone()) {
                return Err(Error::InvalidInput);
            }
            if self.entry(name)?.capability != Capability::DesktopApplication {
                return Err(Error::WrongCapability);
            }
        }
        state.system_packages.sort();
        if self
            .entry(&state.services.postgresql.package_id)?
            .capability
            != Capability::Postgresql17
        {
            return Err(Error::WrongCapability);
        }
        // V1 cannot issue a tested transport-safe SSH change yet. The development
        // user/key/port/firewall and installation baseline are outside this schema.
        if !state.services.openssh.enabled || !state.services.openssh.open_firewall {
            return Err(Error::ProtectedTransport);
        }
        let bytes = canonical(&state)?;
        Ok(Compiled {
            digest: hash(&bytes),
            bytes,
            state,
        })
    }
    pub fn prepare(
        &self,
        current: &Compiled,
        intent: Intent,
        observed_data: DatabaseData,
        grants: &PreparationGrants,
    ) -> Result<Preview> {
        let baseline = self.compile_state(current.state.clone())?;
        if baseline.bytes != current.bytes || baseline.digest != current.digest {
            return Err(Error::InvalidInput);
        }
        let mut next = current.state.clone();
        match intent {
            Intent::InstallPackage { package_id } => {
                if self.entry(&package_id)?.capability != Capability::DesktopApplication {
                    return Err(Error::WrongCapability);
                }
                if !next.system_packages.contains(&package_id) {
                    next.system_packages.push(package_id);
                }
            }
            Intent::RemovePackage { package_id } => {
                if self.entry(&package_id)?.capability != Capability::DesktopApplication {
                    return Err(Error::WrongCapability);
                }
                next.system_packages.retain(|p| p != &package_id);
            }
            Intent::SetPostgresql {
                enabled,
                package_id,
            } => {
                next.services.postgresql.enabled = enabled;
                next.services.postgresql.package_id = package_id;
            }
            Intent::SetOpenssh {
                enabled,
                open_firewall,
            } => {
                next.services.openssh = Openssh {
                    enabled,
                    open_firewall,
                };
            }
            Intent::SetPowerPolicy {
                profile_on_ac,
                profile_on_battery,
            } => {
                next.power_policy = PowerPolicy {
                    profile_on_ac,
                    profile_on_battery,
                };
            }
        }
        let next = self.compile_state(next)?;
        let requested = next.state.system_packages.iter().chain(
            next.state
                .services
                .postgresql
                .enabled
                .then_some(&next.state.services.postgresql.package_id),
        );
        for package_id in requested {
            if self.entry(package_id)?.unfree
                && !grants.acknowledged_unfree_ids.contains(package_id)
            {
                return Err(Error::UnfreeAcknowledgementRequired);
            }
        }
        let pg = &next.state.services.postgresql;
        let old_pg = &current.state.services.postgresql;
        if pg.enabled && pg != old_pg && observed_data != DatabaseData::Absent {
            return Err(Error::DataReviewRequired);
        }
        let mut changes = vec![];
        for field in ["system_packages", "services", "power_policy"] {
            let old = serde_json::to_value(&current.state).map_err(|_| Error::InvalidInput)?;
            let new = serde_json::to_value(&next.state).map_err(|_| Error::InvalidInput)?;
            if old[field] != new[field] {
                changes.push(Change {
                    field: field.into(),
                    before: old[field].clone(),
                    after: new[field].clone(),
                });
            }
        }
        let added = next
            .state
            .system_packages
            .iter()
            .filter(|p| !current.state.system_packages.contains(p))
            .cloned()
            .collect();
        let removed: Vec<_> = current
            .state
            .system_packages
            .iter()
            .filter(|p| !next.state.system_packages.contains(p))
            .cloned()
            .collect();
        let pg_changed = pg != old_pg;
        let mut validators = vec![];
        for name in &next.state.system_packages {
            if !current.state.system_packages.contains(name) {
                validators.push(Validator::DesktopCapability {
                    package_id: name.clone(),
                    binaries: self.entry(name)?.binaries.clone(),
                    desktop_ids: self.entry(name)?.desktop_ids.clone(),
                });
            }
        }
        if pg_changed {
            validators.push(if pg.enabled {
                Validator::PostgresqlUnixReadiness
            } else {
                Validator::PostgresqlStopped
            });
        }
        if next.state.power_policy != current.state.power_policy {
            validators.push(Validator::PowerProfileSupportedAndApplied);
        }
        Ok(Preview { schema_version: 1, baseline_manifest_sha256: current.digest.clone(),
            candidate_manifest_sha256: next.digest.clone(), candidate_manifest: next.state,
            lock_sha256: self.content.lock_sha256.clone(), installation_state_version: self.content.installation_state_version.clone(),
            added_packages: added, removed_packages: removed.clone(), changes, validators,
            risk: if current.digest == next.digest { Risk::R0 } else { Risk::R2 },
            user_data_deleted: false, database_data_may_remain: pg_changed,
            retained_dependency_paths: None, candidate_closure: None, download_bytes: None, build_bytes: None,
            reboot_required: None, recovery: Recovery::RestoreExactPriorConfigurationDataMayRemain,
            final_authorization_ready: false, notes: vec!["User profiles and development shells are unmanaged; removing a declaration preserves user data.".into(),
                "Closure dependencies, reboot effects and build/download costs require build evidence; unknown quantities are not zero.".into()] })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ManagedState {
    pub schema_version: u32,
    pub base_template_revision: String,
    pub catalog_revision: String,
    #[serde(default)]
    pub system_packages: Vec<String>,
    #[serde(default)]
    pub services: Services,
    #[serde(default)]
    pub power_policy: PowerPolicy,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Services {
    #[serde(default)]
    pub postgresql: Postgresql,
    #[serde(default)]
    pub openssh: Openssh,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Postgresql {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "pg_id")]
    pub package_id: String,
    #[serde(default)]
    pub listen_mode: ListenMode,
}
fn pg_id() -> String {
    "postgresql-17".into()
}
impl Default for Postgresql {
    fn default() -> Self {
        Self {
            enabled: false,
            package_id: pg_id(),
            listen_mode: ListenMode::UnixOnly,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ListenMode {
    #[default]
    UnixOnly,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Openssh {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "yes")]
    pub open_firewall: bool,
}
fn yes() -> bool {
    true
}
impl Default for Openssh {
    fn default() -> Self {
        Self {
            enabled: true,
            open_firewall: true,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PowerProfile {
    Performance,
    Balanced,
    PowerSaver,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PowerPolicy {
    #[serde(default = "balanced")]
    pub profile_on_ac: PowerProfile,
    #[serde(default = "saver")]
    pub profile_on_battery: PowerProfile,
}
fn balanced() -> PowerProfile {
    PowerProfile::Balanced
}
fn saver() -> PowerProfile {
    PowerProfile::PowerSaver
}
impl Default for PowerPolicy {
    fn default() -> Self {
        Self {
            profile_on_ac: balanced(),
            profile_on_battery: saver(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Compiled {
    pub state: ManagedState,
    pub bytes: Vec<u8>,
    pub digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Intent {
    InstallPackage {
        package_id: String,
    },
    RemovePackage {
        package_id: String,
    },
    SetPostgresql {
        enabled: bool,
        package_id: String,
    },
    SetOpenssh {
        enabled: bool,
        open_firewall: bool,
    },
    SetPowerPolicy {
        profile_on_ac: PowerProfile,
        profile_on_battery: PowerProfile,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseData {
    Unknown,
    Absent,
    Present,
}
/// Populated by deterministic policy from a real permission receipt; never model
/// text. This is preparatory acknowledgement only, not final activation approval.
#[derive(Default)]
pub struct PreparationGrants {
    pub acknowledged_unfree_ids: BTreeSet<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub field: String,
    pub before: serde_json::Value,
    pub after: serde_json::Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Validator {
    DesktopCapability {
        package_id: String,
        binaries: Vec<String>,
        desktop_ids: Vec<String>,
    },
    PostgresqlUnixReadiness,
    PostgresqlStopped,
    PowerProfileSupportedAndApplied,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Risk {
    R0,
    R2,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recovery {
    RestoreExactPriorConfigurationDataMayRemain,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    pub schema_version: u32,
    pub baseline_manifest_sha256: String,
    pub candidate_manifest_sha256: String,
    pub candidate_manifest: ManagedState,
    pub lock_sha256: String,
    pub installation_state_version: String,
    pub added_packages: Vec<String>,
    pub removed_packages: Vec<String>,
    pub changes: Vec<Change>,
    pub validators: Vec<Validator>,
    pub risk: Risk,
    pub user_data_deleted: bool,
    pub database_data_may_remain: bool,
    pub retained_dependency_paths: Option<Vec<String>>,
    pub candidate_closure: Option<String>,
    pub download_bytes: Option<u64>,
    pub build_bytes: Option<u64>,
    pub reboot_required: Option<bool>,
    pub recovery: Recovery,
    pub final_authorization_ready: bool,
    pub notes: Vec<String>,
}
