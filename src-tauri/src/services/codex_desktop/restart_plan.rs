//! Private, deterministic restart planning for the Codex desktop service.
//!
//! The plan is deliberately backend-only. It contains exact installation and
//! runtime evidence so that the renderer can never choose a process, package,
//! bundle, path, or launch command. Its public-to-the-service methods expose
//! only the opaque capability binding and safe lifecycle groups.

use std::{cmp::Ordering, collections::HashSet};

use sha2::{Digest, Sha256};

use crate::codex_desktop::{
    platform::{TrustedInstallationCandidate, TrustedRuntimeInstance},
    types::CodexDesktopRestartPromptReason,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestartPlanReason {
    UniqueRuntime,
    MultipleInstances,
    MultipleInstallations,
    IdentityBindingAmbiguous,
}

impl RestartPlanReason {
    pub(crate) const fn prompt_reason(self) -> CodexDesktopRestartPromptReason {
        match self {
            Self::UniqueRuntime => CodexDesktopRestartPromptReason::UniqueRuntime,
            Self::MultipleInstances => CodexDesktopRestartPromptReason::MultipleInstances,
            Self::MultipleInstallations => CodexDesktopRestartPromptReason::MultipleInstallations,
            Self::IdentityBindingAmbiguous => {
                CodexDesktopRestartPromptReason::IdentityBindingAmbiguous
            }
        }
    }
}

/// One exact installation and the exact runtime instances currently bound to
/// it. `identity_binding_ambiguous` intentionally withholds a close set: it
/// can be shown in the initial confirmation reason but prevents an execution
/// from guessing a target by name, title, or path.
#[derive(Debug, Clone)]
pub(crate) struct RestartInstallationRuntime {
    pub(crate) candidate: TrustedInstallationCandidate,
    pub(crate) instances: Vec<TrustedRuntimeInstance>,
    pub(crate) identity_binding_ambiguous: bool,
}

/// Fully enumerated, deterministic restart request. All fields remain inside
/// the Rust process; the IPC result derives only an opaque token and a small
/// reason enum from it.
#[derive(Debug, Clone)]
pub(crate) struct RestartPlan {
    pub(crate) app_identity: String,
    pub(crate) installations: Vec<RestartInstallationRuntime>,
    pub(crate) selected_installation: String,
    pub(crate) runtime_instances: Vec<TrustedRuntimeInstance>,
    pub(crate) reason: RestartPlanReason,
    pub(crate) plan_revision: String,
}

impl RestartPlan {
    pub(crate) fn new(mut installations: Vec<RestartInstallationRuntime>) -> Option<Self> {
        if installations.is_empty() {
            return None;
        }

        // A platform is not allowed to make enumeration order a product rule.
        // De-duplicate the exact stable installation key before applying the
        // fixed comparator; duplicated records otherwise could cause one app
        // to be force-closed more than once.
        installations.sort_by(compare_installations);
        installations
            .dedup_by(|left, right| left.candidate.stable_key == right.candidate.stable_key);

        let selected_installation = installations
            .first()
            .expect("non-empty restart installations have a first candidate")
            .candidate
            .stable_key
            .clone();
        let app_identity = installations[0]
            .candidate
            .application
            .stable_identity
            .clone();
        let identity_binding_ambiguous = installations
            .iter()
            .any(|installation| installation.identity_binding_ambiguous);
        let runtime_instances = deduplicate_instances(
            installations
                .iter()
                .flat_map(|installation| installation.instances.iter().cloned()),
        );
        let reason = if identity_binding_ambiguous {
            RestartPlanReason::IdentityBindingAmbiguous
        } else if installations.len() > 1 {
            RestartPlanReason::MultipleInstallations
        } else if runtime_instances.len() > 1 {
            RestartPlanReason::MultipleInstances
        } else {
            RestartPlanReason::UniqueRuntime
        };

        let plan_revision = revision_for(
            &app_identity,
            &installations,
            &selected_installation,
            &runtime_instances,
            reason,
        );

        Some(Self {
            app_identity,
            installations,
            selected_installation,
            runtime_instances,
            reason,
            plan_revision,
        })
    }

    pub(crate) fn prompt_reason(&self) -> CodexDesktopRestartPromptReason {
        self.reason.prompt_reason()
    }

    pub(crate) fn is_not_running(&self) -> bool {
        self.runtime_instances.is_empty()
            && !self
                .installations
                .iter()
                .any(|installation| installation.identity_binding_ambiguous)
    }

    pub(crate) fn has_identity_binding_ambiguity(&self) -> bool {
        self.installations
            .iter()
            .any(|installation| installation.identity_binding_ambiguous)
    }

    pub(crate) fn selected(&self) -> &TrustedInstallationCandidate {
        &self
            .installations
            .iter()
            .find(|installation| installation.candidate.stable_key == self.selected_installation)
            .expect("restart plan selected key always belongs to its candidates")
            .candidate
    }

    /// Returns lifecycle groups whose runtime evidence is unique across every
    /// candidate. A process belonging to more than one malformed platform
    /// record is still offered to the platform force adapter only once.
    pub(crate) fn close_targets(&self) -> Vec<RestartInstallationRuntime> {
        let mut seen = HashSet::new();
        self.installations
            .iter()
            .filter_map(|installation| {
                let instances = installation
                    .instances
                    .iter()
                    .filter(|instance| seen.insert(instance.restart_identity_key()))
                    .cloned()
                    .collect::<Vec<_>>();
                (!instances.is_empty()).then(|| RestartInstallationRuntime {
                    candidate: installation.candidate.clone(),
                    instances,
                    identity_binding_ambiguous: false,
                })
            })
            .collect()
    }
}

fn compare_installations(
    left: &RestartInstallationRuntime,
    right: &RestartInstallationRuntime,
) -> Ordering {
    // Exact requirement: running desc, platform version desc, system before
    // user, stable identity key asc. Any non-comparable structured version is
    // treated as tied; stable key still makes the result deterministic.
    left.instances
        .is_empty()
        .cmp(&right.instances.is_empty())
        .then_with(|| {
            right
                .candidate
                .application
                .platform_version
                .compare(&left.candidate.application.platform_version)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| {
            left.candidate
                .scope
                .priority()
                .cmp(&right.candidate.scope.priority())
        })
        .then_with(|| left.candidate.stable_key.cmp(&right.candidate.stable_key))
}

fn deduplicate_instances(
    instances: impl IntoIterator<Item = TrustedRuntimeInstance>,
) -> Vec<TrustedRuntimeInstance> {
    let mut seen = HashSet::new();
    instances
        .into_iter()
        .filter(|instance| seen.insert(instance.restart_identity_key()))
        .collect()
}

fn revision_for(
    app_identity: &str,
    installations: &[RestartInstallationRuntime],
    selected_installation: &str,
    runtime_instances: &[TrustedRuntimeInstance],
    reason: RestartPlanReason,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(app_identity.as_bytes());
    hasher.update([0]);
    hasher.update(selected_installation.as_bytes());
    hasher.update([0]);
    hasher.update(format!("{reason:?}").as_bytes());
    for installation in installations {
        hasher.update(installation.candidate.stable_key.as_bytes());
        hasher.update([installation.candidate.scope.priority()]);
        hasher.update(
            installation
                .candidate
                .application
                .platform_version
                .canonical()
                .as_bytes(),
        );
        hasher.update([u8::from(installation.identity_binding_ambiguous)]);
    }
    for instance in runtime_instances {
        hasher.update(instance.restart_identity_key().as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use crate::codex_desktop::{
        platform::{
            RestartInstallationScope, TrustedInstallationCandidate, TrustedRuntimeInstance,
        },
        types::{CpuArchitecture, InstalledApplication, LaunchTarget, PlatformVersion},
    };

    use super::{RestartInstallationRuntime, RestartPlan, RestartPlanReason};

    fn candidate(
        key: &str,
        version: &str,
        scope: RestartInstallationScope,
    ) -> TrustedInstallationCandidate {
        TrustedInstallationCandidate {
            application: InstalledApplication {
                stable_identity: "OpenAI.Codex".to_owned(),
                display_name: None,
                display_version: Some(version.to_owned()),
                platform_version: PlatformVersion::parse_windows_msix(version).unwrap(),
                architecture: CpuArchitecture::X86_64,
                location: None,
                launch_target: LaunchTarget::WindowsAumid(format!("{key}!App")),
            },
            scope,
            stable_key: key.to_owned(),
        }
    }

    fn instance(process_id: u32) -> TrustedRuntimeInstance {
        TrustedRuntimeInstance::Windows {
            package_family_name: "fixture_pfn".to_owned(),
            process_id,
            creation_time: process_id as u64,
        }
    }

    #[test]
    fn comparator_is_stable_across_enumeration_order() {
        let first = RestartInstallationRuntime {
            candidate: candidate("user-old", "1.0.0.0", RestartInstallationScope::CurrentUser),
            instances: vec![instance(1)],
            identity_binding_ambiguous: false,
        };
        let second = RestartInstallationRuntime {
            candidate: candidate("system-new", "2.0.0.0", RestartInstallationScope::System),
            instances: vec![instance(2)],
            identity_binding_ambiguous: false,
        };
        let third = RestartInstallationRuntime {
            candidate: candidate("system-idle", "9.0.0.0", RestartInstallationScope::System),
            instances: vec![],
            identity_binding_ambiguous: false,
        };

        let forward = RestartPlan::new(vec![first.clone(), second.clone(), third.clone()]).unwrap();
        let reverse = RestartPlan::new(vec![third, first, second]).unwrap();

        assert_eq!(forward.selected_installation, "system-new");
        assert_eq!(forward.selected_installation, reverse.selected_installation);
        assert_eq!(forward.plan_revision, reverse.plan_revision);
        assert_eq!(forward.reason, RestartPlanReason::MultipleInstallations);
    }

    #[test]
    fn duplicate_runtime_evidence_is_force_targeted_once() {
        let duplicate = instance(42);
        let plan = RestartPlan::new(vec![
            RestartInstallationRuntime {
                candidate: candidate("a", "1.0.0.0", RestartInstallationScope::CurrentUser),
                instances: vec![duplicate.clone()],
                identity_binding_ambiguous: false,
            },
            RestartInstallationRuntime {
                candidate: candidate("b", "1.0.0.0", RestartInstallationScope::CurrentUser),
                instances: vec![duplicate],
                identity_binding_ambiguous: false,
            },
        ])
        .unwrap();

        assert_eq!(plan.runtime_instances.len(), 1);
        assert_eq!(
            plan.close_targets()
                .into_iter()
                .flat_map(|target| target.instances)
                .count(),
            1
        );
    }
}
