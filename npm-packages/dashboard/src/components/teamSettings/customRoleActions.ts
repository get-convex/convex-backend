import { RoleStatementAction } from "@convex-dev/platform/managementApi";

// Server-side validation pairs each action with the resource kind it can target
// (e.g. `team:*:token:*` only accepts team-token actions). Listing the same
// pairing here lets the JSON editor flag mismatches inline before submit.
export type ActionCategory =
  | "team"
  | "project"
  | "deployment"
  | "member"
  | "teamToken"
  | "projectToken"
  | "deploymentToken"
  | "customRole"
  | "billing"
  | "oauthApplication"
  | "sso"
  | "integration"
  | "defaultEnvironmentVariable";

export const ACTIONS_BY_CATEGORY: Record<
  ActionCategory,
  RoleStatementAction[]
> = {
  team: [
    "updateTeam",
    "deleteTeam",
    "applyReferralCode",
    "viewTeamAuditLog",
    "viewUsage",
  ],
  billing: [
    "updatePaymentMethod",
    "updateBillingContact",
    "updateBillingAddress",
    "createSubscription",
    "resumeSubscription",
    "cancelSubscription",
    "changeSubscriptionPlan",
    "setSpendingLimit",
    "viewBillingDetails",
    "viewInvoices",
  ],
  oauthApplication: [
    "createOAuthApplication",
    "updateOAuthApplication",
    "deleteOAuthApplication",
    "viewOAuthApplications",
    "generateOAuthClientSecret",
  ],
  sso: ["enableSSO", "disableSSO", "updateSSO", "viewSSO"],
  integration: [
    "viewTeamIntegrations",
    "createTeamIntegrations",
    "updateTeamIntegrations",
    "deleteTeamIntegrations",
    "viewTeamAuditLog",
  ],
  project: [
    "createProject",
    "transferProject",
    "receiveProject",
    "updateProject",
    "deleteProject",
    "viewProjects",
    "updateMemberProjectRole",
  ],
  defaultEnvironmentVariable: [
    "createProjectEnvironmentVariable",
    "updateProjectEnvironmentVariable",
    "deleteProjectEnvironmentVariable",
    "viewProjectEnvironmentVariables",
  ],
  deployment: [
    "createDeployment",
    "receiveDeployment",
    "transferDeployment",
    "deleteDeployment",
    "viewDeployments",
    "updateDeploymentReference",
    "updateDeploymentDashboardEditConfirmation",
    "updateDeploymentExpiresAt",
    "updateDeploymentSendLogsToClient",
    "updateDeploymentClass",
    "updateDeploymentIsDefault",
    "updateDeploymentType",
    "viewDeploymentIntegrations",
    "writeDeploymentIntegrations",
    "createCustomDomain",
    "deleteCustomDomain",
    "viewInsights",
    "createBackups",
    "importBackups",
    "configurePeriodicBackups",
    "disablePeriodicBackups",
    "deleteBackups",
    "viewBackups",
    "downloadBackups",
    "deploy",
    "pauseDeployment",
    "unpauseDeployment",
    "viewEnvironmentVariables",
    "writeEnvironmentVariables",
    "viewLogs",
    "viewMetrics",
    "viewAuditLog",
    "viewData",
    "writeData",
    "actAsUser",
    "runInternalQueries",
    "runInternalMutations",
    "runInternalActions",
    "runTestQuery",
  ],
  member: [
    "inviteMember",
    "cancelMemberInvitation",
    "removeMember",
    "updateMemberRole",
  ],
  teamToken: [
    "createTeamAccessToken",
    "updateTeamAccessToken",
    "deleteTeamAccessToken",
    "viewTeamAccessTokens",
  ],
  projectToken: [
    "createProjectAccessToken",
    "updateProjectAccessToken",
    "deleteProjectAccessToken",
    "viewProjectAccessTokens",
  ],
  deploymentToken: [
    "createDeploymentAccessToken",
    "updateDeploymentAccessToken",
    "deleteDeploymentAccessToken",
    "viewDeploymentAccessTokens",
  ],
  customRole: ["viewCustomRoles"],
};
