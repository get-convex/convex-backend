// Custom-role evaluation for dashboard UI gating.
//
// Mirrors `crates_private/big_brain_lib/src/roles/eval.rs` so that the
// dashboard can locally decide whether the current member's custom roles
// permit a given action against a concrete resource. The parser also
// mirrors `crates_private/big_brain_lib/src/roles/parse.rs` because
// `RoleStatement.resource` is serialized over the wire as the string form
// (e.g. "project:*:deployment:type=prod").
//
// The big-brain server remains the source of truth for authorization;
// keep this in sync with the Rust eval/parse modules when they change.

import type {
  RoleStatement,
  RoleStatementAction,
  RoleStatementActions,
} from "@convex-dev/platform/managementApi";

export type ResourceKind =
  | "team"
  | "project"
  | "deployment"
  | "member"
  | "token"
  | "customRole"
  | "billing"
  | "oauthApplication"
  | "sso"
  | "integration"
  | "defaultEnvironmentVariable";

export type ProjectSelector =
  | { kind: "any" }
  | { kind: "id"; id: number }
  | { kind: "slug"; slug: string };

export type DeploymentSelector =
  | { kind: "any" }
  | { kind: "id"; id: number }
  | { kind: "type"; deploymentType: string }
  | { kind: "creator"; memberId: number };

export type TokenSelector =
  | { kind: "any" }
  | { kind: "creator"; memberId: number };

export type ResourceSegment =
  | { kind: "team" }
  | { kind: "project"; selectors: ProjectSelector[] }
  | { kind: "deployment"; selectors: DeploymentSelector[] }
  | { kind: "member" }
  | { kind: "token"; selectors: TokenSelector[] }
  | { kind: "customRole" }
  | { kind: "billing" }
  | { kind: "oauthApplication" }
  | { kind: "sso" }
  | { kind: "integration" }
  | { kind: "defaultEnvironmentVariable" };

export type ResourceSpecifier = { segments: ResourceSegment[] };

export type ConcreteSegment =
  | { kind: "team" }
  | { kind: "project"; id: number; slug: string }
  | {
      kind: "deployment";
      id: number;
      deploymentType: string;
      creator: number | null;
    }
  | { kind: "member"; memberId: number }
  | {
      kind: "token";
      tokenKind: "team" | "project" | "deployment";
      creator: number | null;
    }
  | { kind: "customRole" }
  | { kind: "billing" }
  | { kind: "oauthApplication" }
  | { kind: "sso" }
  | { kind: "integration" }
  | { kind: "defaultEnvironmentVariable" };

export type ConcreteResource = { segments: ConcreteSegment[] };

export type AccessDecision = "allowed" | "denied";

// --- Parsing ------------------------------------------------------------

const VALID_KINDS: readonly ResourceKind[] = [
  "team",
  "project",
  "deployment",
  "member",
  "token",
  "customRole",
  "billing",
  "oauthApplication",
  "sso",
  "integration",
  "defaultEnvironmentVariable",
];

function parseProjectSelector(s: string): ProjectSelector {
  if (s === "*") return { kind: "any" };
  if (s.startsWith("id=")) {
    const id = Number(s.slice(3));
    if (!Number.isFinite(id)) {
      throw new Error(`Invalid id value: ${s.slice(3)}`);
    }
    return { kind: "id", id };
  }
  if (s.startsWith("slug=")) return { kind: "slug", slug: s.slice(5) };
  throw new Error(`Invalid selector for project resource: ${s}`);
}

function parseDeploymentSelector(s: string): DeploymentSelector {
  if (s === "*") return { kind: "any" };
  if (s.startsWith("id=")) {
    const id = Number(s.slice(3));
    if (!Number.isFinite(id)) {
      throw new Error(`Invalid id value: ${s.slice(3)}`);
    }
    return { kind: "id", id };
  }
  if (s.startsWith("type="))
    return { kind: "type", deploymentType: s.slice(5) };
  if (s.startsWith("creator=")) {
    const memberId = Number(s.slice(8));
    if (!Number.isFinite(memberId)) {
      throw new Error(`Invalid creator ID: ${s.slice(8)}`);
    }
    return { kind: "creator", memberId };
  }
  throw new Error(`Invalid selector for deployment resource: ${s}`);
}

function parseTokenSelector(s: string): TokenSelector {
  if (s === "*") return { kind: "any" };
  if (s.startsWith("creator=")) {
    const memberId = Number(s.slice(8));
    if (!Number.isFinite(memberId)) {
      throw new Error(`Invalid creator ID: ${s.slice(8)}`);
    }
    return { kind: "creator", memberId };
  }
  throw new Error(`Invalid selector for token resource: ${s}`);
}

function parseSegment(
  kind: ResourceKind,
  selectorStr: string,
): ResourceSegment {
  const parts = selectorStr.split(",").map((p) => p.trim());
  switch (kind) {
    case "team":
      if (parts.length !== 1 || parts[0] !== "*") {
        throw new Error(`Team segment only accepts wildcard (*)`);
      }
      return { kind: "team" };
    case "project":
      return { kind: "project", selectors: parts.map(parseProjectSelector) };
    case "deployment":
      return {
        kind: "deployment",
        selectors: parts.map(parseDeploymentSelector),
      };
    case "member":
      if (parts.length !== 1 || parts[0] !== "*") {
        throw new Error(`Member segment only accepts wildcard (*)`);
      }
      return { kind: "member" };
    case "token":
      return { kind: "token", selectors: parts.map(parseTokenSelector) };
    case "customRole":
    case "billing":
    case "oauthApplication":
    case "sso":
    case "integration":
    case "defaultEnvironmentVariable":
      if (parts.length !== 1 || parts[0] !== "*") {
        throw new Error(`${kind} segment only accepts wildcard (*)`);
      }
      return { kind };
    default: {
      const _exhaustive: never = kind;
      throw new Error(`Unknown resource kind: ${_exhaustive}`);
    }
  }
}

export function parseResourceSpecifier(s: string): ResourceSpecifier {
  const parts = s.split(":");
  if (parts.length % 2 !== 0) {
    throw new Error(`Resource specifier must have kind:selector pairs: ${s}`);
  }
  const segments: ResourceSegment[] = [];
  for (let i = 0; i < parts.length; i += 2) {
    const rawKind = parts[i];
    if (!VALID_KINDS.includes(rawKind as ResourceKind)) {
      throw new Error(`Unknown resource kind: ${rawKind}`);
    }
    segments.push(parseSegment(rawKind as ResourceKind, parts[i + 1]));
  }
  return { segments };
}

// --- Action → resource kind --------------------------------------------

// Mirrors `RoleStatementAction::resource_kind()` in
// `crates_private/big_brain_lib/src/roles/types.rs`. Keep in sync.
const ACTION_RESOURCE_KIND: Record<RoleStatementAction, ResourceKind> = {
  // Team
  "team:update": "team",
  "team:delete": "team",
  "team:applyReferralCode": "team",
  "team:auditLog:view": "team",
  "team:usage:view": "team",
  // Billing
  "billing:paymentMethod:update": "billing",
  "billing:contact:update": "billing",
  "billing:address:update": "billing",
  "billing:subscription:create": "billing",
  "billing:subscription:resume": "billing",
  "billing:subscription:cancel": "billing",
  "billing:subscription:changePlan": "billing",
  "billing:spendingLimit:set": "billing",
  "billing:view": "billing",
  "billing:invoices:view": "billing",
  // OAuth Applications
  "oauthApplication:create": "oauthApplication",
  "oauthApplication:update": "oauthApplication",
  "oauthApplication:delete": "oauthApplication",
  "oauthApplication:view": "oauthApplication",
  "oauthApplication:generateClientSecret": "oauthApplication",
  // SSO
  "sso:enable": "sso",
  "sso:disable": "sso",
  "sso:update": "sso",
  "sso:view": "sso",
  // Team Integrations
  "integration:view": "integration",
  "integration:create": "integration",
  "integration:update": "integration",
  "integration:delete": "integration",
  // Project
  "project:create": "project",
  "project:transfer": "project",
  "project:receive": "project",
  "project:update": "project",
  "project:delete": "project",
  "project:view": "project",
  "project:updateMemberRole": "project",
  // Default (project-scoped) Environment Variables
  "defaultEnvironmentVariable:create": "defaultEnvironmentVariable",
  "defaultEnvironmentVariable:update": "defaultEnvironmentVariable",
  "defaultEnvironmentVariable:delete": "defaultEnvironmentVariable",
  "defaultEnvironmentVariable:view": "defaultEnvironmentVariable",
  // Deployment
  "deployment:create": "deployment",
  "deployment:receive": "deployment",
  "deployment:transfer": "deployment",
  "deployment:updateReference": "deployment",
  "deployment:updateDashboardEditConfirmation": "deployment",
  "deployment:updateExpiresAt": "deployment",
  "deployment:updateSendLogsToClient": "deployment",
  "deployment:updateClass": "deployment",
  "deployment:updateIsDefault": "deployment",
  "deployment:updateType": "deployment",
  "deployment:delete": "deployment",
  "deployment:view": "deployment",
  "deployment:integrations:view": "deployment",
  "deployment:integrations:write": "deployment",
  "deployment:customDomain:create": "deployment",
  "deployment:customDomain:delete": "deployment",
  "deployment:insights:view": "deployment",
  "deployment:backups:create": "deployment",
  "deployment:backups:import": "deployment",
  "deployment:backups:configurePeriodic": "deployment",
  "deployment:backups:disablePeriodic": "deployment",
  "deployment:backups:delete": "deployment",
  "deployment:backups:view": "deployment",
  "deployment:deploy": "deployment",
  "deployment:env:view": "deployment",
  "deployment:env:write": "deployment",
  "deployment:pause": "deployment",
  "deployment:unpause": "deployment",
  "deployment:logs:view": "deployment",
  "deployment:metrics:view": "deployment",
  "deployment:data:view": "deployment",
  "deployment:data:write": "deployment",
  "deployment:backups:download": "deployment",
  "deployment:functions:actAsUser": "deployment",
  "deployment:functions:runInternalQueries": "deployment",
  "deployment:functions:runInternalMutations": "deployment",
  "deployment:functions:runInternalActions": "deployment",
  "deployment:functions:runTestQuery": "deployment",
  "deployment:auditLog:view": "deployment",
  // Member
  "member:invite": "member",
  "member:cancelInvitation": "member",
  "member:remove": "member",
  "member:updateRole": "member",
  // Custom Role
  "customRole:view": "customRole",
  // Token
  "team:token:create": "token",
  "team:token:update": "token",
  "team:token:delete": "token",
  "team:token:view": "token",
  "project:token:create": "token",
  "project:token:update": "token",
  "project:token:delete": "token",
  "project:token:view": "token",
  "deployment:token:create": "token",
  "deployment:token:update": "token",
  "deployment:token:delete": "token",
  "deployment:token:view": "token",
};

export function actionResourceKind(action: RoleStatementAction): ResourceKind {
  return ACTION_RESOURCE_KIND[action];
}

// --- Selector matching --------------------------------------------------

function projectSelectorMatches(
  selector: ProjectSelector,
  segment: { id: number; slug: string },
): boolean {
  switch (selector.kind) {
    case "any":
      return true;
    case "id":
      return segment.id === selector.id;
    case "slug":
      return segment.slug === selector.slug;
    default: {
      const _exhaustive: never = selector;
      return _exhaustive;
    }
  }
}

function deploymentSelectorMatches(
  selector: DeploymentSelector,
  segment: { id: number; deploymentType: string; creator: number | null },
): boolean {
  switch (selector.kind) {
    case "any":
      return true;
    case "id":
      return segment.id === selector.id;
    case "type":
      return segment.deploymentType === selector.deploymentType;
    case "creator":
      return segment.creator === selector.memberId;
    default: {
      const _exhaustive: never = selector;
      return _exhaustive;
    }
  }
}

function tokenSelectorMatches(
  selector: TokenSelector,
  segment: { creator: number | null },
): boolean {
  switch (selector.kind) {
    case "any":
      return true;
    case "creator":
      return segment.creator === selector.memberId;
    default: {
      const _exhaustive: never = selector;
      return _exhaustive;
    }
  }
}

function segmentMatches(
  spec: ResourceSegment,
  concrete: ConcreteSegment,
): boolean {
  if (spec.kind !== concrete.kind) return false;
  switch (spec.kind) {
    case "project":
      if (concrete.kind !== "project") return false;
      return spec.selectors.some((s) => projectSelectorMatches(s, concrete));
    case "deployment":
      if (concrete.kind !== "deployment") return false;
      return spec.selectors.some((s) => deploymentSelectorMatches(s, concrete));
    case "token":
      if (concrete.kind !== "token") return false;
      return spec.selectors.some((s) => tokenSelectorMatches(s, concrete));
    default:
      return true;
  }
}

function specifierMatches(
  spec: ResourceSpecifier,
  concrete: ConcreteResource,
): boolean {
  if (spec.segments.length !== concrete.segments.length) return false;
  return spec.segments.every((seg, i) =>
    segmentMatches(seg, concrete.segments[i]),
  );
}

// --- Evaluation ---------------------------------------------------------

function actionMatches(
  pattern: RoleStatementActions,
  action: RoleStatementAction,
): boolean {
  if (pattern === "*") return true;
  return pattern.includes(action);
}

/**
 * Evaluate whether the given collection of role statements grants `action`
 * on `resource`. Mirrors `evaluate_statements` in `eval.rs`:
 *
 *   1. The resource's leaf kind must match the action's resource kind, else
 *      Denied (the action would never apply to this resource shape).
 *   2. A matching `Deny` short-circuits to Denied.
 *   3. Otherwise, a matching `Allow` produces Allowed.
 *   4. Default Denied.
 */
export function evaluateStatements(
  statements: readonly RoleStatement[],
  action: RoleStatementAction,
  resource: ConcreteResource,
): AccessDecision {
  const leaf = resource.segments[resource.segments.length - 1];
  if (!leaf || leaf.kind !== actionResourceKind(action)) {
    return "denied";
  }

  let anyAllow = false;
  for (const statement of statements) {
    if (!actionMatches(statement.actions, action)) continue;
    let spec: ResourceSpecifier;
    try {
      spec = parseResourceSpecifier(statement.resource);
    } catch {
      // Malformed statements never match; the server-side validation should
      // prevent these from being persisted in the first place.
      continue;
    }
    if (!specifierMatches(spec, resource)) continue;
    if (statement.effect === "deny") return "denied";
    anyAllow = true;
  }
  return anyAllow ? "allowed" : "denied";
}

/**
 * Roles are additive across the list: `Allow` from any role permits the
 * action even if a different role would have denied it. Within a single
 * role, `Deny` still overrides `Allow` (per `evaluateStatements`). Mirrors
 * `allowed_deployment_ops` in `eval.rs`.
 */
export function evaluateRoles(
  roles: readonly { statements: readonly RoleStatement[] }[],
  action: RoleStatementAction,
  resource: ConcreteResource,
): AccessDecision {
  for (const role of roles) {
    if (evaluateStatements(role.statements, action, resource) === "allowed") {
      return "allowed";
    }
  }
  return "denied";
}
