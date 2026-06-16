use common::types::MemberId;
use errors::ErrorMetadata;
use keybroker::{
    bad_admin_key_error,
    DeploymentOp,
    Identity,
};

use super::types::{
    ActionPattern,
    ConcreteDeployment,
    ConcreteProject,
    ConcreteResource,
    ConcreteSegment,
    ConcreteToken,
    CustomRole,
    DeploymentSelector,
    ProjectSelector,
    ResourceSegment,
    ResourceSpecifier,
    RolePolicyAction,
    RoleStatementAction,
    RoleStatementEffect,
    TokenSelector,
};

/// The result of evaluating a custom role against an action and resource.
#[derive(Debug, PartialEq, Eq)]
pub enum AccessDecision {
    Allowed,
    Denied,
}

impl ProjectSelector {
    fn matches(&self, project: &ConcreteProject) -> bool {
        match self {
            ProjectSelector::Any => true,
            ProjectSelector::Id(id) => project.id == *id,
            ProjectSelector::Slug(slug) => project.slug == *slug,
        }
    }
}

impl DeploymentSelector {
    fn matches(&self, deployment: &ConcreteDeployment, actor: MemberId) -> bool {
        match self {
            DeploymentSelector::Any => true,
            DeploymentSelector::Id(id) => deployment.id == *id,
            DeploymentSelector::Type(t) => deployment.deployment_type == *t,
            DeploymentSelector::Creator(c) => deployment.creator == Some(c.resolve(actor)),
        }
    }
}

impl TokenSelector {
    fn matches(&self, token: &ConcreteToken, actor: MemberId) -> bool {
        match self {
            TokenSelector::Any => true,
            TokenSelector::Creator(c) => token.creator == Some(c.resolve(actor)),
        }
    }
}

impl ResourceSegment {
    /// Returns true if this segment matches the given concrete segment.
    /// Multiple selectors within a segment are OR'd — any match suffices.
    /// `actor` is the member id of the principal whose permissions are
    /// being evaluated; used to resolve `creator=self` selectors.
    pub(crate) fn matches(&self, concrete: &ConcreteSegment, actor: MemberId) -> bool {
        match (self, concrete) {
            (ResourceSegment::Team, ConcreteSegment::Team) => true,
            (ResourceSegment::Project(selectors), ConcreteSegment::Project(project)) => {
                selectors.iter().any(|s| s.matches(project))
            },
            (ResourceSegment::Project(selectors), ConcreteSegment::ProposedProject { slug }) => {
                selectors.iter().any(|s| match s {
                    ProjectSelector::Any => true,
                    ProjectSelector::Id(_) => false,
                    ProjectSelector::Slug(s) => s == slug,
                })
            },
            (ResourceSegment::Deployment(selectors), ConcreteSegment::Deployment(deployment)) => {
                selectors.iter().any(|s| s.matches(deployment, actor))
            },
            (
                ResourceSegment::Deployment(selectors),
                ConcreteSegment::ProposedDeployment {
                    deployment_type,
                    creator,
                },
            ) => selectors.iter().any(|s| match s {
                DeploymentSelector::Any => true,
                DeploymentSelector::Id(_) => false,
                DeploymentSelector::Type(t) => t == deployment_type,
                DeploymentSelector::Creator(c) => *creator == Some(c.resolve(actor)),
            }),
            (ResourceSegment::Member, ConcreteSegment::Member) => true,
            (ResourceSegment::Token(selectors), ConcreteSegment::Token(token)) => {
                selectors.iter().any(|s| s.matches(token, actor))
            },
            (ResourceSegment::CustomRole, ConcreteSegment::CustomRole) => true,
            (ResourceSegment::Billing, ConcreteSegment::Billing) => true,
            (ResourceSegment::OauthApplication, ConcreteSegment::OauthApplication) => true,
            (ResourceSegment::Sso, ConcreteSegment::Sso) => true,
            (ResourceSegment::Integration, ConcreteSegment::Integration) => true,
            (
                ResourceSegment::DefaultEnvironmentVariable,
                ConcreteSegment::DefaultEnvironmentVariable,
            ) => true,
            // Mismatched kinds never match.
            _ => false,
        }
    }
}

impl ResourceSpecifier {
    /// Returns true if this specifier matches the given concrete resource.
    /// Requires exact segment count match (no parent-matches-child).
    fn matches(&self, resource: &ConcreteResource, actor: MemberId) -> bool {
        if self.segments.len() != resource.segments.len() {
            return false;
        }
        self.segments
            .iter()
            .zip(resource.segments.iter())
            .all(|(spec_seg, concrete_seg)| spec_seg.matches(concrete_seg, actor))
    }
}

impl CustomRole {
    /// Evaluate whether this role grants the given action on the given
    /// resource.
    ///
    /// `actor` is the member id of the principal whose permissions are being
    /// evaluated, used to resolve `creator=self` selectors against the
    /// resource's creator.
    ///
    /// Uses deny-overrides-allow on a default-deny baseline:
    /// 1. If any matching rule has effect Deny, the result is Denied.
    /// 2. If at least one matching rule has effect Allow (and none deny), the
    ///    result is Allowed.
    /// 3. If no rules match, the result is Denied.
    pub fn evaluate(
        &self,
        action: &RolePolicyAction,
        resource: &ConcreteResource,
        actor: MemberId,
    ) -> AccessDecision {
        // `*CustomRole` actions are intentionally ungrantable by a custom role
        // (see `RolePolicyAction::to_statement_action`); they always deny.
        let Some(stmt_action) = action.to_statement_action() else {
            return AccessDecision::Denied;
        };
        evaluate_statements(self.statements.iter(), stmt_action, resource, actor)
    }
}

/// Same deny-overrides-allow eval used by [`CustomRole::evaluate`], but driven
/// by a [`RoleStatementAction`] so it can be applied to actions that have no
/// [`RolePolicyAction`] counterpart yet, and by an iterator so it can flatten
/// statements across multiple roles.
fn evaluate_statements<'a>(
    statements: impl IntoIterator<Item = &'a super::types::RoleStatement>,
    action: RoleStatementAction,
    resource: &ConcreteResource,
    actor: MemberId,
) -> AccessDecision {
    let leaf_kind = resource.segments.last().map(|s| s.kind());
    if leaf_kind != Some(action.resource_kind()) {
        return AccessDecision::Denied;
    }

    let mut any_allow = false;
    for rule in statements {
        let action_match = match &rule.actions {
            ActionPattern::Wildcard => true,
            ActionPattern::Specific(actions) => actions.contains(&action),
        };
        if action_match && rule.resource.matches(resource, actor) {
            match rule.effect {
                RoleStatementEffect::Deny => return AccessDecision::Denied,
                RoleStatementEffect::Allow => any_allow = true,
            }
        }
    }

    if any_allow {
        AccessDecision::Allowed
    } else {
        AccessDecision::Denied
    }
}

/// Every [`DeploymentOp`] except `Unknown`, in the same order as the variant
/// declaration. Used to enumerate ops when computing what a set of roles
/// allows on a deployment.
pub const ALL_DEPLOYMENT_OPS: &[DeploymentOp] = &[
    DeploymentOp::Deploy,
    DeploymentOp::ViewEnvironmentVariables,
    DeploymentOp::WriteEnvironmentVariables,
    DeploymentOp::PauseDeployment,
    DeploymentOp::UnpauseDeployment,
    DeploymentOp::ViewLogs,
    DeploymentOp::ViewMetrics,
    DeploymentOp::ViewIntegrations,
    DeploymentOp::WriteIntegrations,
    DeploymentOp::ViewData,
    DeploymentOp::WriteData,
    DeploymentOp::ViewBackups,
    DeploymentOp::CreateBackups,
    DeploymentOp::DownloadBackups,
    DeploymentOp::DeleteBackups,
    DeploymentOp::ImportBackups,
    DeploymentOp::ActAsUser,
    DeploymentOp::RunInternalQueries,
    DeploymentOp::RunInternalMutations,
    DeploymentOp::RunInternalActions,
    DeploymentOp::RunTestQuery,
    DeploymentOp::ViewAuditLog,
];

/// Authoritative mapping from a keybroker [`DeploymentOp`] to the
/// [`RoleStatementAction`] that gates it.
pub fn deployment_op_action(op: DeploymentOp) -> Option<RoleStatementAction> {
    use DeploymentOp as O;
    use RoleStatementAction as A;
    Some(match op {
        O::Deploy => A::Deploy,
        O::ViewEnvironmentVariables => A::ViewEnvironmentVariables,
        O::WriteEnvironmentVariables => A::WriteEnvironmentVariables,
        O::PauseDeployment => A::PauseDeployment,
        O::UnpauseDeployment => A::UnpauseDeployment,
        O::ViewLogs => A::ViewLogs,
        O::ViewMetrics => A::ViewMetrics,
        O::ViewIntegrations => A::ViewDeploymentIntegrations,
        O::WriteIntegrations => A::WriteDeploymentIntegrations,
        O::ViewData => A::ViewData,
        O::WriteData => A::WriteData,
        O::ViewBackups => A::ViewBackups,
        O::CreateBackups => A::CreateBackups,
        O::DownloadBackups => A::DownloadBackups,
        O::DeleteBackups => A::DeleteBackups,
        O::ImportBackups => A::ImportBackups,
        O::ActAsUser => A::ActAsUser,
        O::RunInternalQueries => A::RunInternalQueries,
        O::RunInternalMutations => A::RunInternalMutations,
        O::RunInternalActions => A::RunInternalActions,
        O::RunTestQuery => A::RunTestQuery,
        O::ViewAuditLog => A::ViewAuditLog,
        O::Unknown => return None,
    })
}

pub trait RequireDeploymentOp {
    fn require_operation(&self, operation: DeploymentOp) -> anyhow::Result<()>;
}

impl RequireDeploymentOp for Identity {
    /// Check that this identity is an admin allowed to perform `operation`.
    /// System identities are always allowed. Admin identities are checked
    /// against their allowed operations. All other identities are rejected.
    fn require_operation(&self, operation: DeploymentOp) -> anyhow::Result<()> {
        let admin_identity = match self {
            Identity::System(_) => return Ok(()),
            Identity::DeploymentAdmin(admin_identity) | Identity::ActingUser(admin_identity, _) => {
                admin_identity
            },
            Identity::User(_) | Identity::Unknown(_) => {
                return Err(bad_admin_key_error(self.instance_name()).into());
            },
        };
        if !admin_identity.is_operation_allowed(operation)? {
            let action = deployment_op_action(operation)
                .map_or_else(|| format!("{operation:?}"), |action| action.to_string());
            anyhow::bail!(ErrorMetadata::forbidden(
                "Unauthorized",
                format!("You do not have permission to perform this operation ({action})."),
            ));
        }
        Ok(())
    }
}

/// Returns the [`DeploymentOp`]s that `roles` collectively allow on
/// `deployment` (which lives under `project`). Roles are additive: an op is
/// allowed if *any* role evaluates to `Allowed` for it. Within a single role,
/// `Deny` still overrides `Allow` (per [`CustomRole::evaluate`]), but a `Deny`
/// in one role does not override an `Allow` in another.
pub fn allowed_deployment_ops(
    roles: &[CustomRole],
    project: &ConcreteProject,
    deployment: &ConcreteDeployment,
    actor: MemberId,
) -> Vec<DeploymentOp> {
    let resource = ConcreteResource {
        segments: vec![
            ConcreteSegment::Project(project.clone()),
            ConcreteSegment::Deployment(deployment.clone()),
        ],
    };
    allowed_deployment_ops_for_resource(roles, &resource, actor)
}

/// Same as [`allowed_deployment_ops`] but evaluates against an
/// already-built [`ConcreteResource`]. Used by the deploy-key escalation
/// guard, which needs to evaluate ops against synthesized
/// `ProposedDeployment` segments under a project.
pub fn allowed_deployment_ops_for_resource(
    roles: &[CustomRole],
    resource: &ConcreteResource,
    actor: MemberId,
) -> Vec<DeploymentOp> {
    ALL_DEPLOYMENT_OPS
        .iter()
        .copied()
        .filter(|op| {
            let Some(action) = deployment_op_action(*op) else {
                return false;
            };
            roles.iter().any(|role| {
                evaluate_statements(role.statements.iter(), action, resource, actor)
                    == AccessDecision::Allowed
            })
        })
        .collect()
}
