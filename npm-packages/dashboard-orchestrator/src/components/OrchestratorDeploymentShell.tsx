// Mounts dashboard-common's DeploymentInfoProvider + DeploymentApiProvider +
// DeploymentDashboardLayout for an orchestrator-managed deployment route.
//
// The URL is `/t/<team>/<project>/<deploymentName>/...` so the shared
// `DeploymentApiProvider` (which reads `router.query.deploymentName`)
// connects to the right deployment automatically. We only need to provide
// `DeploymentInfoContext` with the right adminKey + deploymentUrl + the
// URI prefixes the layout uses to build sidebar links.

import { useRouter } from "next/router";
import { ReactNode, useMemo } from "react";
import useSWR from "swr";
import { ErrorBoundary } from "./ErrorBoundary";
import { DeploymentDashboardLayout } from "@common/layouts/DeploymentDashboardLayout";
import {
  DeploymentApiProvider,
  DeploymentInfo,
  DeploymentInfoContext,
  WaitForDeploymentApi,
} from "@common/lib/deploymentContext";
import { deploymentUrlForBrowser } from "@common/lib/deploymentUrl";
import udfs from "@common/udfs";
import { useQuery } from "convex/react";
import { Tooltip } from "@ui/Tooltip";
import Link from "next/link";
import { LoadingLogo } from "@ui/Loading";
import { useAccessToken } from "../lib/useOrchestratorToken";
import { orchestratorRegionName, orchestratorUrl } from "../lib/config";
import { shouldForwardDeploymentCaptureMessage } from "../lib/deploymentMessages";
import {
  fetchDeploymentAuth,
  listDeployments,
  listProjects,
  listTeams,
} from "../lib/orchestratorApi";
import { stubWorkOsOperations } from "../lib/deploymentInfo";

export function OrchestratorDeploymentShell({
  children,
}: {
  children: ReactNode;
}) {
  const router = useRouter();
  const teamSlug = router.query.team as string | undefined;
  const projectSlug = router.query.project as string | undefined;
  const deploymentName = router.query.deploymentName as string | undefined;
  const token = useAccessToken();
  const url = orchestratorUrl();
  const regionName = orchestratorRegionName();

  const { data: teams } = useSWR(token ? ["teams", token] : null, () =>
    listTeams(url, token!),
  );
  const team = useMemo(
    () => teams?.find((t) => t.slug === teamSlug),
    [teams, teamSlug],
  );
  const { data: projects } = useSWR(
    team && token ? ["projects", team.id, token] : null,
    () => listProjects(url, token!, team!.id),
  );
  const project = useMemo(
    () => projects?.find((p) => p.slug === projectSlug),
    [projects, projectSlug],
  );
  const { data: deployments } = useSWR(
    project && token ? ["deployments", project.id, token] : null,
    () => listDeployments(url, token!, project!.id),
  );
  const deployment = useMemo(
    () => deployments?.find((d) => d.name === deploymentName),
    [deployments, deploymentName],
  );

  // Mint an admin key for this deployment via the orchestrator.
  const { data: auth } = useSWR(
    deployment && token ? ["deploymentAuth", deployment.name, token] : null,
    () => fetchDeploymentAuth(url, token!, deployment!.name),
  );

  if (!team || !project || !deployment || !auth || !token) {
    return (
      <div className="flex size-full items-center justify-center">
        <LoadingLogo />
      </div>
    );
  }

  // The orchestrator's ephemeral_admin_key route falls back to minting a
  // session-bound PAT when the deployment row doesn't have a real backend
  // admin key (`<deploymentName>|<secret>` format) on file. The Convex backend
  // rejects PAT-formatted keys for system queries with "BadAdminKey", so the
  // page would crash on the first useQuery. Detect that case up-front and
  // render a friendly state instead.
  const looksLikeRealAdminKey = auth.adminKey.startsWith(`${deployment.name}|`);
  if (!looksLikeRealAdminKey) {
    return (
      <NoBackendState
        teamSlug={team.slug}
        projectSlug={project.slug}
        deploymentName={deployment.name}
      />
    );
  }

  const teamsURI = `/t/${team.slug}`;
  const projectsURI = `/t/${team.slug}/${project.slug}`;
  const deploymentsURI = `/t/${team.slug}/${project.slug}/${deployment.name}`;
  const browserDeploymentUrl = deploymentUrlForBrowser(auth.url);

  const deploymentInfo: DeploymentInfo = {
    ok: true,
    adminKey: auth.adminKey,
    deploymentUrl: browserDeploymentUrl,
    addBreadcrumb: () => {},
    captureMessage: (message, severity) => {
      if (shouldForwardDeploymentCaptureMessage(message, severity)) {
        console.error(message, severity);
      }
    },
    captureException: console.error,
    reportHttpError: (method, url2, error) =>
      console.error(`HTTP ${method} ${url2}: ${error.code} ${error.message}`),
    useCurrentTeam: () => ({ id: team.id, name: team.name, slug: team.slug }),
    useTeamMembers: () => [],
    useTeamEntitlements: () => ({
      auditLogRetentionDays: 365,
      logStreamingEnabled: true,
      streamingExportEnabled: true,
    }),
    useTeamUsageState: () => "Default",
    useCurrentUsageBanner: () => null,
    useCurrentProject: () => ({
      id: project.id,
      name: project.name,
      slug: project.slug,
    }),
    // Shape this as a cloud-kind PlatformDeploymentResponse so the shared
    // `<DeploymentSummary>` renders the URL panel (gated on `kind === "cloud"`).
    // The orchestrator stores type/class on differently named fields; remap
    // them here rather than forking the summary component.
    useCurrentDeployment: () =>
      ({
        kind: "cloud",
        id: deployment.id,
        projectId: deployment.projectId,
        name: deployment.name,
        // `reference` is the human-readable identifier the cloud dashboard
        // displays in bold before the deployment name. The orchestrator
        // doesn't track one, so reuse the deployment type — it gives
        // `<DeploymentSummary>` non-empty content for the bold span.
        reference:
          deployment.deploymentType ?? deployment.kind ?? deployment.name,
        deploymentType: (deployment.deploymentType ??
          deployment.kind ??
          "prod") as "prod" | "dev" | "preview" | "custom",
        // Surface the resource tier (S4/S8/S16/... or custom) as the deployment's
        // `class` so the summary card badge shows something meaningful for
        // orchestrator deployments. Falls back to "S16" if the orchestrator
        // pre-dates the tier-on-platform-response field.
        class: deployment.tier ?? "S16",
        deploymentUrl: browserDeploymentUrl,
        createTime: deployment.creationTime,
        region: regionName,
        isDefault: true,
        previewIdentifier: deployment.previewIdentifier ?? null,
        // PlatformDeploymentResponse has more optional fields the dashboard
        // doesn't read; cast through unknown to satisfy the type checker.
      }) as unknown as ReturnType<DeploymentInfo["useCurrentDeployment"]>,
    useIsProtectedDeployment: () =>
      deployment.kind === "prod" || deployment.deploymentType === "prod",
    useHasProjectAdminPermissions: () => true,
    useIsOperationAllowed: () => true,
    useHasCustomRole: () => false,
    useIsDeploymentPaused: () => {
      const state = useQuery(udfs.deploymentState.deploymentState);
      return state?.state === "paused";
    },
    useProjectEnvironmentVariables: () => ({ configs: [] }),
    useLogDeploymentEvent: () => () => {},
    workOSOperations: stubWorkOsOperations,
    CloudImport: ({ sourceCloudBackupId }: { sourceCloudBackupId: number }) => (
      <div>{sourceCloudBackupId}</div>
    ),
    TeamMemberLink: () => (
      <Tooltip tip="Identity is managed by the orchestrator's auth provider.">
        <span className="underline decoration-dotted underline-offset-4">
          A team member
        </span>
      </Tooltip>
    ),
    Link,
    ErrorBoundary: ({ children: ec }: { children: React.ReactNode }) => (
      <ErrorBoundary>{ec}</ErrorBoundary>
    ),
    // DeploymentInfoContext expects a JSX-returning component, so fall back
    // to an empty fragment with a single noop child rather than `null`.
    DisconnectOverlay: () => <>{null}</>,
    teamsURI,
    projectsURI,
    deploymentsURI,
    // The orchestrator dashboard is the cloud chrome (multi-deployment, with
    // teams/projects/audit) talking to a self-hosted control plane instead of
    // BigBrain. dashboard-common's `isSelfHosted` flag selects between cloud
    // chrome and the single-deployment self-hosted shell — orchestrator wants
    // the cloud chrome, so this stays false.
    isSelfHosted: false,
    // ...but the backend behind every orchestrator deployment IS a
    // convex-local-backend that owns its own admin keys via `/api/admin_keys`.
    // Surface the Admin Keys page in the deployment settings sidebar without
    // flipping `isSelfHosted` (which would disable Project Settings / Usage
    // links and break the chrome).
    deploymentBackendOwnsAdminKeys: true,
    workosIntegrationEnabled: false,
    logStreamTopicFiltersEnabled: true,
    connectionStateCheckIntervalMs: 2500,
  };

  return (
    <DeploymentInfoContext.Provider value={deploymentInfo}>
      <DeploymentApiProvider>
        <WaitForDeploymentApi>
          <DeploymentDashboardLayout>
            {children as JSX.Element}
          </DeploymentDashboardLayout>
        </WaitForDeploymentApi>
      </DeploymentApiProvider>
    </DeploymentInfoContext.Provider>
  );
}

function NoBackendState({
  teamSlug,
  projectSlug,
  deploymentName,
}: {
  teamSlug: string;
  projectSlug: string;
  deploymentName: string;
}) {
  return (
    <main className="flex flex-1 flex-col items-center justify-center gap-4 p-8 text-center">
      {/* eslint-disable-next-line no-restricted-syntax -- decorative icon glyph, not a heading */}
      <div className="flex size-16 items-center justify-center rounded-full bg-background-tertiary text-2xl text-content-secondary">
        ⚡
      </div>
      <div className="flex max-w-xl flex-col gap-2">
        {/* eslint-disable-next-line no-restricted-syntax -- text-xl IS the header style on an h1 */}
        <h1 className="text-xl font-semibold text-content-primary">
          No backend connected
        </h1>
        <p className="text-sm text-content-secondary">
          The deployment{" "}
          <code className="rounded-sm bg-background-tertiary px-1 font-mono text-xs">
            {deploymentName}
          </code>{" "}
          doesn&apos;t have a running Convex backend. The orchestrator
          provisioned the row but can&apos;t mint an admin key for it.
        </p>
        <p className="text-sm text-content-secondary">
          Connect a backend container by setting its admin key in the
          orchestrator&apos;s database, or remove this deployment via Project
          Settings.
        </p>
      </div>
      <div className="flex items-center gap-2">
        <Link
          href={`/t/${teamSlug}/${projectSlug}/settings`}
          className="inline-flex items-center rounded-md border bg-background-secondary px-3 py-1.5 text-sm hover:bg-background-tertiary"
        >
          Open Project Settings
        </Link>
        <Link
          href={`/t/${teamSlug}/${projectSlug}`}
          className="inline-flex items-center rounded-md border bg-background-secondary px-3 py-1.5 text-sm hover:bg-background-tertiary"
        >
          Back to Deployments
        </Link>
      </div>
    </main>
  );
}
