//! Exact command descriptions for the locked NixOS activation interface.
//! This module does not spawn commands or admit a client-provided argv.
use crate::{Candidate, Effect, Error, Plan, Result};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
pub struct CommandSpec {
    pub executable: String,
    pub args: Vec<String>,
    pub clear_environment: bool,
    pub environment: BTreeMap<String, String>,
}
#[derive(Debug)]
pub struct RetainedNix {
    executable: String,
}
impl RetainedNix {
    /// Runtime integration must also verify readonly root ownership and the
    /// installed binary's digest, then retain its closure against collection.
    pub fn from_store_package(package: &str) -> Result<Self> {
        if !crate::valid_store(package) {
            return Err(Error::InvalidPlan);
        }
        Ok(Self {
            executable: format!("{package}/bin/nix-env"),
        })
    }
}
pub fn command(plan: &Plan, effect: &Effect, nix: &RetainedNix) -> Result<Option<CommandSpec>> {
    plan.validate()?;
    let candidate = match &plan.candidate {
        Candidate::System { closure } => Some(closure),
        _ => None,
    };
    let selected = match effect {
        Effect::TestSystem { closure } if candidate.is_some_and(|c| &c.path == closure) => Some((
            format!("{closure}/bin/switch-to-configuration"),
            vec!["test".into()],
        )),
        Effect::RecoverSystem { closure }
            if closure == &plan.prior.running.path && candidate.is_some() =>
        {
            Some((
                format!("{closure}/bin/switch-to-configuration"),
                vec!["test".into()],
            ))
        }
        Effect::SetProfile { closure }
            if candidate.is_some()
                && (closure == &plan.prior.profile.path
                    || candidate.is_some_and(|c| &c.path == closure)) =>
        {
            Some((
                nix.executable.clone(),
                vec![
                    "--profile".into(),
                    "/nix/var/nix/profiles/system".into(),
                    "--set".into(),
                    closure.clone(),
                ],
            ))
        }
        Effect::InstallBoot { closure }
            if candidate.is_some()
                && (closure == &plan.prior.boot.path
                    || candidate.is_some_and(|c| &c.path == closure)) =>
        {
            Some((
                format!("{closure}/bin/switch-to-configuration"),
                vec!["boot".into()],
            ))
        }
        Effect::TestSystem { .. }
        | Effect::RecoverSystem { .. }
        | Effect::SetProfile { .. }
        | Effect::InstallBoot { .. } => return Err(Error::InvalidPlan),
        _ => None,
    };
    Ok(selected.map(|(executable, args)| CommandSpec {
        executable,
        args,
        clear_environment: true,
        environment: BTreeMap::from([
            ("HOME".into(), "/root".into()),
            ("LANG".into(), "C.UTF-8".into()),
        ]),
    }))
}
