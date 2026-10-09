//! Fixed native mechanism; no caller subject, action, details or approved flag.
use super::{
    Binding,
    policy::{self, InstalledPolicy},
};
use crate::{
    Error, Result,
    caller::{ServiceIdentity, SystemBus, VerifiedCaller},
};
use std::{collections::HashMap, sync::{Arc, atomic::{AtomicBool, Ordering}},
    thread, time::Duration};
use zbus::{blocking::Proxy, zvariant::Value};

const PATH: &str = "/org/freedesktop/PolicyKit1/Authority";
const INTERFACE: &str = "org.freedesktop.PolicyKit1.Authority";
pub(crate) type ActionDescription = (
    String,
    String,
    String,
    String,
    String,
    String,
    u32,
    u32,
    u32,
    HashMap<String, String>,
);
type AuthorizationResult = (bool, bool, HashMap<String, String>);
pub(crate) struct NativeAuthentication {
    pub owner: ServiceIdentity,
}
fn subject(caller: &VerifiedCaller) -> (&str, HashMap<&str, Value<'_>>) {
    (
        "system-bus-name",
        HashMap::from([("name", Value::from(caller.identity().sender.as_str()))]),
    )
}
fn fresh_result(result: &AuthorizationResult) -> bool {
    result.0
        && !result.1
        && !result.2.contains_key("polkit.temporary_authorization_id")
        && !result
            .2
            .contains_key("polkit.retains_authorization_after_challenge")
        && !result.2.contains_key("polkit.dismissed")
}
pub(crate) fn descriptions(proxy: &Proxy<'_>) -> Result<Vec<ActionDescription>> {
    let descriptions: Vec<ActionDescription> = proxy
        .call("EnumerateActions", &("C",))
        .map_err(|_| Error::AuthRequired)?;
    if descriptions.len() > 8192
        || descriptions
            .iter()
            .any(|a| a.0.len() > 256 || a.9.len() > 128)
    {
        return Err(Error::Integrity);
    }
    Ok(descriptions)
}
fn verify_action(descriptions: &[ActionDescription], action: &str) -> Result<()> {
    if !matches!(action, policy::ACTION | policy::ELEVATED_ACTION) {
        return Err(Error::Invalid);
    }
    let mut matches = descriptions.iter().filter(|a| a.0 == action);
    let a = matches.next().ok_or(Error::AuthRequired)?;
    // Exactly auth_admin, with no implied actions, pkexec route or cached defaults.
    if matches.next().is_some() || (a.6, a.7, a.8) != (2, 2, 2) || !a.9.is_empty() {
        return Err(Error::Integrity);
    }
    Ok(())
}
pub(crate) fn authenticate(
    bus: &SystemBus,
    caller: &VerifiedCaller,
    policy: &InstalledPolicy,
    binding: &Binding,
    cancelled: &Arc<AtomicBool>,
) -> Result<NativeAuthentication> {
    bus.recheck(caller)?;
    let owner = bus.polkit_owner(&policy.authority)?;
    let connection = bus.polkit_connection()?;
    let proxy = Proxy::new(&connection, owner.sender.as_str(), PATH, INTERFACE)
        .map_err(|_| Error::AuthRequired)?;
    verify_action(&descriptions(&proxy)?, &binding.action)?;
    let details = HashMap::from([
        ("horizon.plan_id", binding.plan_id.as_str()),
        ("horizon.plan_hash", binding.plan_hash.as_str()),
        ("horizon.closure", binding.closure.as_str()),
        ("horizon.policy_revision", binding.policy_revision.as_str()),
        (
            "horizon.installation_uuid",
            binding.target.installation_uuid.as_str(),
        ),
    ]);
    // Cached/implicit authorization cannot stand in for a fresh native challenge.
    let prior: AuthorizationResult = proxy
        .call(
            "CheckAuthorization",
            &(subject(caller), binding.action.as_str(), &details, 0u32, ""),
        )
        .map_err(|_| Error::AuthRequired)?;
    if prior.0
        || !prior.1
        || prior.2.contains_key("polkit.temporary_authorization_id")
        || prior
            .2
            .contains_key("polkit.retains_authorization_after_challenge")
    {
        return Err(Error::AuthRequired);
    }
    let cancellation = uuid::Uuid::new_v4().to_string();
    let completed = Arc::new(AtomicBool::new(false));
    let watcher_completed = completed.clone();
    let watcher_cancelled = cancelled.clone();
    let watcher_connection = connection.clone();
    let watcher_owner = owner.sender.clone();
    let watcher_cancellation = cancellation.clone();
    let watcher = thread::spawn(move || {
        while !watcher_completed.load(Ordering::Acquire) {
            if watcher_cancelled.load(Ordering::Acquire) {
                if let Ok(proxy) = Proxy::new(
                    &watcher_connection, watcher_owner.as_str(), PATH, INTERFACE) {
                    let _: zbus::Result<()> = proxy.call(
                        "CancelCheckAuthorization", &(watcher_cancellation.as_str(),));
                }
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
    });
    let response: std::result::Result<AuthorizationResult, _> = proxy.call(
        "CheckAuthorization",
        &(
            subject(caller),
            binding.action.as_str(),
            &details,
            1u32,
            cancellation.as_str(),
        ),
    );
    completed.store(true, Ordering::Release);
    watcher.join().map_err(|_| Error::AuthRequired)?;
    let response = match response {
        Ok(response) => response,
        Err(_) => {
            // Cancellation is best-effort; timeout never creates an authority proof.
            let _: zbus::Result<()> =
                proxy.call_noreply("CancelCheckAuthorization", &(cancellation.as_str(),));
            return Err(Error::AuthRequired);
        }
    };
    if cancelled.load(Ordering::Acquire) { return Err(Error::AuthRequired); }
    if !fresh_result(&response) {
        return Err(Error::AuthRequired);
    }
    bus.recheck(caller)?;
    if bus.polkit_owner(&policy.authority)? != owner
        || InstalledPolicy::load(bus.target())?.revision != policy.revision
    {
        return Err(Error::TargetChanged);
    }
    drop(proxy);
    Ok(NativeAuthentication { owner })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_dismissed_or_challenge_responses_are_not_fresh_authentication() {
        assert!(fresh_result(&(true, false, HashMap::new())));
        for result in [
            (false, false, HashMap::new()),
            (true, true, HashMap::new()),
            (false, true, HashMap::new()),
        ] {
            assert!(!fresh_result(&result));
        }
        for key in [
            "polkit.temporary_authorization_id",
            "polkit.retains_authorization_after_challenge",
            "polkit.dismissed",
        ] {
            assert!(!fresh_result(&(
                true,
                false,
                HashMap::from([(key.into(), "".into())])
            )));
        }
    }
    #[test]
    fn only_declared_uncached_admin_actions_with_no_implication_are_accepted() {
        let action = |defaults| {
            (
                policy::ACTION.into(),
                "".into(),
                "".into(),
                "Horizon OS".into(),
                "".into(),
                "".into(),
                defaults,
                defaults,
                defaults,
                HashMap::new(),
            )
        };
        assert_eq!(verify_action(&[action(2)], policy::ACTION), Ok(()));
        for default in [0, 1, 3, 4, 5] {
            assert!(verify_action(&[action(default)], policy::ACTION).is_err());
        }
        let mut implied = action(2);
        implied.9.insert(
            "org.freedesktop.policykit.imply".into(),
            "other.action".into(),
        );
        assert!(verify_action(&[implied], policy::ACTION).is_err());
        assert!(verify_action(&[action(2), action(2)], policy::ACTION).is_err());
        assert!(verify_action(&[action(2)], "org.freedesktop.policykit.exec").is_err());
    }
    #[test]
    fn actual_polkit_availability_is_checked_and_absence_cannot_authorize() {
        let connection = crate::caller::read_only_test_connection().unwrap();
        let bus = Proxy::new(
            &connection,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .unwrap();
        let activatable: Vec<String> = bus.call("ListActivatableNames", &()).unwrap();
        let available = activatable
            .iter()
            .any(|name| name == "org.freedesktop.PolicyKit1");
        let owned: bool = bus
            .call("NameHasOwner", &("org.freedesktop.PolicyKit1",))
            .unwrap();
        if !available && !owned {
            assert_eq!(
                crate::caller::start_polkit(&connection),
                Err(Error::AuthRequired)
            );
            println!(
                "AIOS_POLKIT_OBSERVATIONS {}",
                serde_json::json!({"evidence_kind":"actual-guest-polkit-unavailable-and-fail-closed",
                "service_available":false,"owner_identity_verified":false,
                "boot_id":std::fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim(),
                "native_authorization_verified":false,"trusted_ui_confirmation_verified":false,"check_authorization_called":false})
            );
            return;
        }
        crate::caller::start_polkit(&connection).unwrap();
        let unique: String = bus
            .call("GetNameOwner", &("org.freedesktop.PolicyKit1",))
            .unwrap();
        let credentials: zbus::fdo::ConnectionCredentials = bus
            .call("GetConnectionCredentials", &(unique.as_str(),))
            .unwrap();
        let uid = credentials.unix_user_id().unwrap();
        let owner =
            crate::caller::observe_service(&connection, "org.freedesktop.PolicyKit1", uid).unwrap();
        assert_eq!(owner.pid, credentials.process_id().unwrap());
        let proxy = Proxy::new(&connection, owner.sender.as_str(), PATH, INTERFACE).unwrap();
        let actions = descriptions(&proxy).unwrap();
        assert!(!actions.is_empty());
        let horizon: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a.0.as_str(), policy::ACTION | policy::ELEVATED_ACTION))
            .collect();
        for a in &horizon {
            verify_action(&actions, &a.0).unwrap();
        }
        let version: String = proxy.get_property("BackendVersion").unwrap();
        assert_eq!(
            crate::caller::observe_service(&connection, "org.freedesktop.PolicyKit1", uid).unwrap(),
            owner
        );
        println!(
            "AIOS_POLKIT_OBSERVATIONS {}",
            serde_json::json!({"evidence_kind":"actual-guest-read-only-polkit-owner-and-action-catalog",
            "service_available":true,"owner_identity_verified":true,"owner":owner.sender,"uid":owner.uid,"pid":owner.pid,"start_ticks":owner.start_ticks,"boot_id":owner.boot_id,
            "backend_version":version,"registered_action_count":actions.len(),"horizon_actions_installed":horizon.len(),
            "native_authorization_verified":false,"trusted_ui_confirmation_verified":false,"check_authorization_called":false})
        );
    }
}
