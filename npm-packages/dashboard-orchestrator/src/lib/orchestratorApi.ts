// HTTP client for the convex-orchestrator API surface. Mirrors the route
// shapes the orchestrator (`crates/orchestrator`) exposes:
//
//   POST /api/authorize                     - login
//   GET  /api/dashboard/profile             - current member
//   GET  /api/dashboard/teams               - teams
//   POST /api/dashboard/teams               - create team
//   GET  /api/dashboard/teams/{id}/projects - projects in team
//   POST /api/create_project                - create project
//   GET  /v1/projects/{id}/list_deployments - deployments
//   POST /v1/projects/{id}/create_deployment- provision deployment
//   POST /api/dashboard/instances/{name}/auth - mint deployment admin key

import { z } from "zod";

// ---------- Schemas ----------

export const memberSchema = z.object({
  id: z.number(),
  email: z.string(),
  name: z.string().nullable(),
});
export type Member = z.infer<typeof memberSchema>;

export const teamSchema = z.object({
  id: z.number(),
  name: z.string(),
  slug: z.string(),
  creator: z.number().nullable().optional(),
});
export type Team = z.infer<typeof teamSchema>;

export const projectSchema = z.object({
  id: z.number(),
  teamId: z.number(),
  name: z.string(),
  slug: z.string(),
  isDemo: z.boolean(),
  creationTime: z.number(),
});
export type Project = z.infer<typeof projectSchema>;

export const deploymentSchema = z.object({
  id: z.number(),
  projectId: z.number(),
  name: z.string(),
  kind: z.string().optional(),
  deploymentType: z.string().optional(),
  deploymentClass: z.string().optional(),
  url: z.string(),
  siteUrl: z.string(),
  state: z.string(),
  creationTime: z.number(),
  region: z.string().nullable().optional(),
  previewIdentifier: z.string().nullable().optional(),
  // Optional for backward compat with orchestrator builds that pre-date the
  // tier-on-platform-response field. Defaults to "S16" downstream when absent.
  tier: z.string().optional(),
});
export type Deployment = z.infer<typeof deploymentSchema>;

// ---------- Errors ----------

export class OrchestratorApiError extends Error {
  status: number;
  code?: string;
  constructor(status: number, message: string, code?: string) {
    super(message);
    this.status = status;
    this.code = code;
  }
}

// ---------- Internals ----------

async function request<T>(
  baseUrl: string,
  path: string,
  init: RequestInit & { auth?: boolean; token?: string | null } = {},
): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Accept: "application/json",
    ...((init.headers as Record<string, string>) ?? {}),
  };
  const useAuth = init.auth !== false;
  if (useAuth && init.token) {
    headers.Authorization = `Bearer ${init.token}`;
  }
  const url = `${baseUrl.replace(/\/$/, "")}${path}`;
  const res = await fetch(url, { ...init, headers });
  if (!res.ok) {
    let message = res.statusText;
    let code: string | undefined;
    try {
      const body = (await res.json()) as { code?: string; message?: string };
      message = body.message ?? message;
      code = body.code;
    } catch {
      /* ignore */
    }
    throw new OrchestratorApiError(res.status, message, code);
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

// ---------- Public API ----------

export type AuthorizeResponse = {
  accessToken: string;
  memberId: number;
};

export async function authorizeWithBootstrapToken(
  baseUrl: string,
  bootstrapToken: string,
  deviceName = "dashboard-orchestrator",
): Promise<AuthorizeResponse> {
  return request<AuthorizeResponse>(baseUrl, "/api/authorize", {
    method: "POST",
    auth: false,
    body: JSON.stringify({ deviceName, bootstrapToken }),
  });
}

export async function authorizeWithPassword(
  baseUrl: string,
  email: string,
  password: string,
  deviceName = "dashboard-orchestrator",
): Promise<AuthorizeResponse> {
  return request<AuthorizeResponse>(baseUrl, "/api/authorize", {
    method: "POST",
    auth: false,
    body: JSON.stringify({ deviceName, email, password }),
  });
}

export async function getProfile(
  baseUrl: string,
  token: string,
): Promise<Member> {
  return memberSchema.parse(
    await request<unknown>(baseUrl, "/api/dashboard/profile", { token }),
  );
}

export async function listTeams(
  baseUrl: string,
  token: string,
): Promise<Team[]> {
  const data = await request<unknown>(baseUrl, "/api/dashboard/teams", {
    token,
  });
  return z.array(teamSchema).parse(data);
}

export async function createTeam(
  baseUrl: string,
  token: string,
  name: string,
): Promise<Team> {
  const data = await request<unknown>(baseUrl, "/api/dashboard/teams", {
    method: "POST",
    token,
    body: JSON.stringify({ name }),
  });
  return teamSchema.parse(data);
}

export async function listProjects(
  baseUrl: string,
  token: string,
  teamId: number,
): Promise<Project[]> {
  const data = await request<unknown>(
    baseUrl,
    `/api/dashboard/teams/${teamId}/projects`,
    { token },
  );
  return z.array(projectSchema).parse(data);
}

export type CreateProjectResponse = {
  projectId: number;
  projectSlug: string;
  teamSlug: string;
  deploymentName: string | null;
  url: string | null;
  adminKey: string | null;
};

export async function createProject(
  baseUrl: string,
  token: string,
  teamSlug: string,
  projectName: string,
  deploymentType: "prod" | "dev" | null = "prod",
  tier?: string,
  knobOverrides?: Record<string, string>,
  provisioningMode?: "default" | "volume-sqlite" | "sidecar",
): Promise<CreateProjectResponse> {
  return request<CreateProjectResponse>(baseUrl, "/api/create_project", {
    method: "POST",
    token,
    body: JSON.stringify({
      team: teamSlug,
      projectName,
      deploymentType,
      tier,
      provisioningMode,
      knobOverrides,
    }),
  });
}

export async function listDeployments(
  baseUrl: string,
  token: string,
  projectId: number,
): Promise<Deployment[]> {
  const data = await request<unknown>(
    baseUrl,
    `/v1/projects/${projectId}/list_deployments`,
    { token },
  );
  return z.array(deploymentSchema).parse(data);
}

// Team-level listing — single round trip for the deployments tab on the
// team home page. Backed by GET /v1/teams/{team_id}/list_deployments.
export async function listDeploymentsForTeam(
  baseUrl: string,
  token: string,
  teamId: number,
): Promise<Deployment[]> {
  const data = await request<{ deployments: unknown[] }>(
    baseUrl,
    `/v1/teams/${teamId}/list_deployments`,
    { token },
  );
  return z.array(deploymentSchema).parse(data.deployments ?? data);
}

export async function createDeployment(
  baseUrl: string,
  token: string,
  projectId: number,
  kind: "prod" | "dev" | "preview",
): Promise<Deployment> {
  const data = await request<unknown>(
    baseUrl,
    `/v1/projects/${projectId}/create_deployment`,
    {
      method: "POST",
      token,
      body: JSON.stringify({ kind }),
    },
  );
  return deploymentSchema.parse(data);
}

export type DeploymentAuth = {
  adminKey: string;
  url: string;
};

export async function fetchDeploymentAuth(
  baseUrl: string,
  token: string,
  deploymentName: string,
): Promise<DeploymentAuth> {
  return request<DeploymentAuth>(
    baseUrl,
    `/api/dashboard/instances/${deploymentName}/auth`,
    { method: "POST", token },
  );
}

// ---------- Project settings / host capacity / knob registry ----------

export const projectSettingsResponseSchema = z.object({
  tier: z.string(),
  knobOverrides: z.record(z.string(), z.string()),
});
export type ProjectSettings = z.infer<typeof projectSettingsResponseSchema>;

export async function getProjectSettings(
  baseUrl: string,
  token: string,
  projectId: number,
): Promise<ProjectSettings> {
  const data = await request<unknown>(
    baseUrl,
    `/v1/projects/${projectId}/settings`,
    { token },
  );
  return projectSettingsResponseSchema.parse(data);
}

export async function patchProjectSettings(
  baseUrl: string,
  token: string,
  projectId: number,
  patch: {
    tier?: string;
    knobOverrides?: Record<string, string | null>;
  },
): Promise<ProjectSettings> {
  const data = await request<unknown>(
    baseUrl,
    `/v1/projects/${projectId}/settings`,
    {
      method: "PATCH",
      token,
      body: JSON.stringify(patch),
    },
  );
  return projectSettingsResponseSchema.parse(data);
}

export const hostCapacityResponseSchema = z.object({
  totalMemoryMb: z.number(),
  totalCpus: z.number(),
  allocatedMemoryMb: z.number(),
  allocatedCpus: z.number(),
  deploymentCount: z.number(),
});
export type HostCapacity = z.infer<typeof hostCapacityResponseSchema>;

export async function getHostCapacity(
  baseUrl: string,
  token: string,
): Promise<HostCapacity> {
  const data = await request<unknown>(baseUrl, "/api/dashboard/host_capacity", {
    token,
  });
  return hostCapacityResponseSchema.parse(data);
}

export const knobEntrySchema = z.object({
  envVar: z.string(),
  description: z.string(),
  category: z.string(),
  exposure: z.enum(["curated", "tierTuned", "advanced"]),
  displayName: z.string().nullable(),
  defaultValue: z
    .string()
    .nullable()
    .optional()
    .transform((value) => value ?? null),
});
export type KnobEntry = z.infer<typeof knobEntrySchema>;

export async function getKnobRegistry(
  baseUrl: string,
  token: string,
): Promise<KnobEntry[]> {
  const data = await request<{ knobs: unknown[] }>(
    baseUrl,
    "/api/dashboard/knob_registry",
    { token },
  );
  return z.array(knobEntrySchema).parse(data.knobs);
}

// ---------- Deployment-level settings / restart ----------

export const deploymentSettingsResponseSchema = z.object({
  effectiveTier: z.string(),
  desiredTier: z.string().nullable(),
  desiredOverrides: z.record(z.string(), z.string()),
  runningTier: z.string(),
  runningOverrides: z.record(z.string(), z.string()),
});
export type DeploymentSettings = z.infer<
  typeof deploymentSettingsResponseSchema
>;

export async function getDeploymentSettings(
  baseUrl: string,
  token: string,
  deploymentName: string,
): Promise<DeploymentSettings> {
  const data = await request<unknown>(
    baseUrl,
    `/v1/deployments/${encodeURIComponent(deploymentName)}/settings`,
    { token },
  );
  return deploymentSettingsResponseSchema.parse(data);
}

export async function patchDeploymentSettings(
  baseUrl: string,
  token: string,
  deploymentName: string,
  patch: {
    // `undefined` = leave unchanged, `null` = clear (fall back to project
    // tier), string = set as override.
    desiredTier?: string | null;
    desiredOverrides?: Record<string, string | null>;
  },
): Promise<DeploymentSettings> {
  const body: Record<string, unknown> = {};
  if (patch.desiredTier !== undefined) body.desiredTier = patch.desiredTier;
  if (patch.desiredOverrides !== undefined)
    body.desiredOverrides = patch.desiredOverrides;
  const data = await request<unknown>(
    baseUrl,
    `/v1/deployments/${encodeURIComponent(deploymentName)}/settings`,
    {
      method: "PATCH",
      token,
      body: JSON.stringify(body),
    },
  );
  return deploymentSettingsResponseSchema.parse(data);
}

export async function restartDeployment(
  baseUrl: string,
  token: string,
  deploymentName: string,
  force?: boolean,
): Promise<void> {
  await request<unknown>(
    baseUrl,
    `/v1/deployments/${encodeURIComponent(deploymentName)}/restart`,
    {
      method: "POST",
      token,
      body: JSON.stringify(force ? { force } : {}),
    },
  );
}

// ---------- Custom domains ----------

export const customDomainSchema = z.object({
  id: z.number(),
  deploymentId: z.number(),
  domain: z.string(),
  certState: z.string(),
  createdAt: z.number(),
  challengeType: z.string(),
  dnsCredentialId: z.number().nullable(),
  lastError: z.string().nullable(),
});

export const dnsProviderFieldSchema = z.object({
  key: z.string(),
  label: z.string(),
  help: z.string(),
});

/** Provider list is served by the orchestrator so adding one there needs no
 * dashboard change. */
export const dnsProviderInfoSchema = z.object({
  provider: z.string(),
  fields: z.array(dnsProviderFieldSchema),
});
export type DnsProviderInfo = z.infer<typeof dnsProviderInfoSchema>;

export const dnsCredentialSchema = z.object({
  id: z.number(),
  name: z.string(),
  provider: z.string(),
  createdAt: z.number(),
});
export type DnsCredential = z.infer<typeof dnsCredentialSchema>;

export const listDnsCredentialsSchema = z.object({
  credentials: z.array(dnsCredentialSchema),
  providers: z.array(dnsProviderInfoSchema),
});
export type ListDnsCredentialsResponse = z.infer<
  typeof listDnsCredentialsSchema
>;
export type CustomDomain = z.infer<typeof customDomainSchema>;

export const listCustomDomainsSchema = z.object({
  domains: z.array(customDomainSchema),
  targetHost: z.string(),
  routingEnabled: z.boolean(),
  providers: z.array(dnsProviderInfoSchema),
});
export type ListCustomDomainsResponse = z.infer<typeof listCustomDomainsSchema>;

export const verifyCustomDomainSchema = z.object({
  domain: z.string(),
  certState: z.string(),
  error: z.string().nullable(),
});
export type VerifyCustomDomainResponse = z.infer<
  typeof verifyCustomDomainSchema
>;

export async function listCustomDomains(
  baseUrl: string,
  token: string,
  deploymentId: number,
): Promise<ListCustomDomainsResponse> {
  const data = await request<unknown>(
    baseUrl,
    `/api/dashboard/deployments/${deploymentId}/custom_domains/list`,
    { token },
  );
  return listCustomDomainsSchema.parse(data);
}

export async function createCustomDomain(
  baseUrl: string,
  token: string,
  deploymentId: number,
  domain: string,
  challengeType: "http-01" | "dns-01" = "http-01",
  dnsCredentialId?: number | null,
): Promise<CustomDomain> {
  const data = await request<unknown>(
    baseUrl,
    `/api/dashboard/deployments/${deploymentId}/custom_domains/create`,
    {
      method: "POST",
      token,
      body: JSON.stringify({
        domain,
        challengeType,
        dnsCredentialId: dnsCredentialId ?? null,
      }),
    },
  );
  return customDomainSchema.parse(data);
}

/** Re-runs issuance for a domain whose last attempt failed. */
export async function retryCustomDomain(
  baseUrl: string,
  token: string,
  deploymentId: number,
  domain: string,
): Promise<void> {
  await request<unknown>(
    baseUrl,
    `/api/dashboard/deployments/${deploymentId}/custom_domains/retry`,
    { method: "POST", token, body: JSON.stringify({ domain }) },
  );
}

export async function listDnsCredentials(
  baseUrl: string,
  token: string,
  teamId: number,
): Promise<ListDnsCredentialsResponse> {
  const data = await request<unknown>(
    baseUrl,
    `/api/dashboard/teams/${teamId}/dns_credentials/list`,
    { token },
  );
  return listDnsCredentialsSchema.parse(data);
}

export async function createDnsCredential(
  baseUrl: string,
  token: string,
  teamId: number,
  name: string,
  provider: string,
  secrets: Record<string, string>,
): Promise<DnsCredential> {
  const data = await request<unknown>(
    baseUrl,
    `/api/dashboard/teams/${teamId}/dns_credentials/create`,
    {
      method: "POST",
      token,
      body: JSON.stringify({ name, provider, secrets }),
    },
  );
  return dnsCredentialSchema.parse(data);
}

export async function deleteDnsCredential(
  baseUrl: string,
  token: string,
  teamId: number,
  credentialId: number,
): Promise<void> {
  await request<unknown>(
    baseUrl,
    `/api/dashboard/teams/${teamId}/dns_credentials/${credentialId}/delete`,
    { method: "POST", token },
  );
}

export async function deleteCustomDomain(
  baseUrl: string,
  token: string,
  deploymentId: number,
  domain: string,
): Promise<void> {
  await request<unknown>(
    baseUrl,
    `/api/dashboard/deployments/${deploymentId}/custom_domains/delete`,
    { method: "POST", token, body: JSON.stringify({ domain }) },
  );
}

export async function verifyCustomDomain(
  baseUrl: string,
  token: string,
  deploymentId: number,
  domain: string,
): Promise<VerifyCustomDomainResponse> {
  const data = await request<unknown>(
    baseUrl,
    `/api/dashboard/deployments/${deploymentId}/custom_domains/verify`,
    { method: "POST", token, body: JSON.stringify({ domain }) },
  );
  return verifyCustomDomainSchema.parse(data);
}
