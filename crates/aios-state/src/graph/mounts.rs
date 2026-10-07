//! Fixed system-mount graph snapshots. Capacity and free space are observations;
//! neither mount paths nor source identities authorize storage effects.
use super::{
    native::{Error, NativeTime, Result},
    store::{GraphStore, Node, ProviderSnapshot, ProviderState, Scope, SourceTruth},
    ObservationTime, SourceRevision,
};
use aios_system::storage::MountStatus;

pub const PROVIDER: &str = "native-system-mounts";

pub struct NativeMountSnapshot {
    captured: ObservationTime,
    mounts: Vec<MountStatus>,
    token: Option<String>,
}

impl NativeMountSnapshot {
    pub fn collect(store: &GraphStore) -> Result<Self> {
        if store.native_scope() != Scope::System { return Err(Error::WrongScope); }
        let result = (|| {
            let captured = NativeTime::observe()?.observation().clone();
            let token = store.reconciliation_plan(PROVIDER.into(), captured.clone(), SourceRevision::default())?.state.map(|state| state.token);
            let snapshot = Self { captured, mounts: aios_system::storage::read_system_mounts()?, token };
            snapshot.fresh()?;
            Ok(snapshot)
        })();
        if result.is_err() { store.report_event_loss(); }
        result
    }

    fn fresh(&self) -> Result<()> {
        let now = NativeTime::observe()?.observation().clone();
        if now.boot != self.captured.boot || now.monotonic_ns.checked_sub(self.captured.monotonic_ns).is_none_or(|age| age > 2_000_000_000) {
            return Err(Error::Expired);
        }
        Ok(())
    }

    pub fn mounts(&self) -> &[MountStatus] { &self.mounts }
    pub fn ids(&self) -> Vec<String> { self.mounts.iter().map(|mount| format!("storage:{}", mount.mount_id)).collect() }

    pub fn apply(&self, store: &GraphStore) -> Result<ProviderState> {
        if store.native_scope() != Scope::System { return Err(Error::WrongScope); }
        let result = (|| {
            self.fresh()?;
            if aios_system::storage::read_system_mounts()? != self.mounts { return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged)); }
            let nodes = self.mounts.iter().map(|mount| {
                let id = format!("storage:{}", mount.mount_id);
                Node {
                    id: id.clone(),
                    kind: "storage_mount".into(),
                    scope: Scope::System,
                    provider: PROVIDER.into(),
                    stable_key: id,
                    properties: serde_json::json!({
                        "mount": mount,
                        "captured": self.captured,
                        "execution_authority": false,
                        "format_or_repair_available": false,
                    }),
                    source_truth: SourceTruth::Running,
                    realtime_ns: self.captured.realtime_ns,
                }
            }).collect();
            let state = store.apply_provider_snapshot(ProviderSnapshot {
                provider: PROVIDER.into(),
                expected_token: self.token.clone(),
                source_truth: SourceTruth::Running,
                time: self.captured.clone(),
                source_revision: SourceRevision::default(),
                complete: true,
                nodes,
                verified_absent_ids: vec![],
            })?;
            if aios_system::storage::read_system_mounts()? != self.mounts { return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged)); }
            self.fresh()?;
            Ok(state)
        })();
        if result.is_err() { store.report_event_loss(); }
        result
    }
}
