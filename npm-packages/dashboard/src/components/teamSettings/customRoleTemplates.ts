import {
  RoleStatement,
  RoleStatementAction,
} from "@convex-dev/platform/managementApi";
import { ACTIONS_BY_CATEGORY } from "./customRoleActions";

export type CustomRoleTemplate = {
  id: string;
  label: string;
  description: string;
  defaultName: string;
  defaultRoleDescription: string;
  statements: RoleStatement[];
};

// Lifecycle and class-changing actions stay admin-only — notably
// `updateDeploymentType`, which would let an editor reclassify a
// non-production deployment as production.
const VIEW_DEPLOYMENT_ACTIONS: RoleStatementAction[] = [
  "viewInsights",
  "viewDeploymentIntegrations",
  "viewEnvironmentVariables",
  "viewLogs",
  "viewMetrics",
  "viewAuditLog",
  "viewData",
];

const EDIT_DEPLOYMENT_ACTIONS: RoleStatementAction[] = [
  "createDeployment",
  "viewInsights",
  "viewDeploymentIntegrations",
  "writeDeploymentIntegrations",
  "viewEnvironmentVariables",
  "writeEnvironmentVariables",
  "viewLogs",
  "viewMetrics",
  "viewAuditLog",
  "viewData",
  "writeData",
  "deploy",
  "pauseDeployment",
  "unpauseDeployment",
  "actAsUser",
  "runInternalQueries",
  "runInternalMutations",
  "runInternalActions",
  "runTestQuery",
  "createCustomDomain",
  "deleteCustomDomain",
  "createBackups",
  "importBackups",
  "configurePeriodicBackups",
  "disablePeriodicBackups",
  "deleteBackups",
  "downloadBackups",
];

export const CUSTOM_ROLE_TEMPLATES: CustomRoleTemplate[] = [
  {
    id: "billing",
    label: "Billing",
    description: "Access to billing and usage.",
    defaultName: "Billing",
    defaultRoleDescription:
      "Manage payment, subscriptions, and view billing and usage.",
    statements: [
      {
        effect: "allow",
        resource: "billing:*",
        actions: ACTIONS_BY_CATEGORY.billing,
      },
      {
        effect: "allow",
        resource: "team:*",
        actions: ["viewUsage"],
      },
    ],
  },
  {
    id: "team-config",
    label: "Team Configuration",
    description:
      "Manage team settings, billing, OAuth, SSO, integrations, and members. No project access.",
    defaultName: "Team Configuration",
    defaultRoleDescription:
      "Configure team settings, billing, OAuth applications, SSO, integrations, and members. No project or deployment access.",
    statements: [
      {
        effect: "allow",
        resource: "team:*",
        actions: ["updateTeam", "viewUsage", "viewTeamAuditLog"],
      },
      {
        effect: "allow",
        resource: "billing:*",
        actions: ACTIONS_BY_CATEGORY.billing,
      },
      {
        effect: "allow",
        resource: "oauthApplication:*",
        actions: ACTIONS_BY_CATEGORY.oauthApplication,
      },
      {
        effect: "allow",
        resource: "sso:*",
        actions: ACTIONS_BY_CATEGORY.sso,
      },
      {
        effect: "allow",
        resource: "integration:*",
        actions: ACTIONS_BY_CATEGORY.integration,
      },
      {
        effect: "allow",
        resource: "member:*",
        actions: ACTIONS_BY_CATEGORY.member,
      },
    ],
  },
  {
    id: "project-admin",
    label: "Admin for all projects",
    description:
      "Full admin on every project and deployment. No team-level settings.",
    defaultName: "Project Admin",
    defaultRoleDescription:
      "Full administrative access to all projects and deployments. No team-level settings.",
    statements: [
      {
        effect: "allow",
        resource: "project:*",
        actions: ACTIONS_BY_CATEGORY.project,
      },
      {
        effect: "allow",
        resource: "project:*:deployment:*",
        actions: ACTIONS_BY_CATEGORY.deployment,
      },
      {
        effect: "allow",
        resource: "project:*:defaultEnvironmentVariable:*",
        actions: ACTIONS_BY_CATEGORY.defaultEnvironmentVariable,
      },
      {
        effect: "allow",
        resource: "project:*:token:*",
        actions: ACTIONS_BY_CATEGORY.projectToken,
      },
      {
        effect: "allow",
        resource: "project:*:deployment:*:token:*",
        actions: ACTIONS_BY_CATEGORY.deploymentToken,
      },
    ],
  },
  {
    id: "view-non-prod",
    label: "View all non-production",
    description:
      "View projects and non-production deployments. No production access.",
    defaultName: "View Non-Production",
    defaultRoleDescription:
      "View all projects and non-production deployments. No access to production deployments.",
    statements: [
      {
        effect: "allow",
        resource: "project:*",
        actions: ["viewProjects"],
      },
      {
        effect: "allow",
        resource: "project:*:deployment:*",
        actions: VIEW_DEPLOYMENT_ACTIONS,
      },
      {
        effect: "deny",
        resource: "project:*:deployment:type=prod",
        actions: "*",
      },
    ],
  },
  {
    id: "edit-non-prod",
    label: "Edit all non-production",
    description:
      "View and edit projects, deploy to non-production. No production access.",
    defaultName: "Edit Non-Production",
    defaultRoleDescription:
      "View and edit all projects, deploy to non-production deployments. No access to production deployments.",
    statements: [
      {
        effect: "allow",
        resource: "project:*",
        actions: ["viewProjects", "updateProject", "createProject"],
      },
      {
        effect: "allow",
        resource: "project:*:deployment:*",
        actions: EDIT_DEPLOYMENT_ACTIONS,
      },
      {
        effect: "allow",
        resource: "project:*:defaultEnvironmentVariable:*",
        actions: ACTIONS_BY_CATEGORY.defaultEnvironmentVariable,
      },
      {
        effect: "deny",
        resource: "project:*:deployment:type=prod",
        actions: "*",
      },
    ],
  },
];

export const CUSTOM_ROLE_TEMPLATES_BY_ID: Record<string, CustomRoleTemplate> =
  Object.fromEntries(CUSTOM_ROLE_TEMPLATES.map((t) => [t.id, t]));
