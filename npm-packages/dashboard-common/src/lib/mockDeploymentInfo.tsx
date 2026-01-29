import { DeploymentInfo } from "@common/lib/deploymentContext";

export const mockDeploymentInfo: DeploymentInfo = {
  ok: true,
  reportHttpError: () => {},
  captureException: () => {},
  captureMessage: () => {},
  addBreadcrumb: () => {},
  deploymentUrl: process.env.NEXT_PUBLIC_DEPLOYMENT_URL!,
  adminKey: process.env.NEXT_PUBLIC_ADMIN_KEY!,
  useCurrentTeam: () => ({
    id: 0,
    name: "Team",
    slug: "team",
  }),
  useTeamMembers: () => [],
  useTeamEntitlements: () => ({}),
  useCurrentUsageBanner: () => null,
  useCurrentProject: () => ({
    id: 0,
    name: "Project",
    slug: "project",
    teamId: 0,
  }),
  useLogDeploymentEvent: () => () => {},
  workOSOperations: {
    useDeploymentWorkOSEnvironment: () => undefined,
    useTeamWorkOSIntegration: () => undefined,
    useWorkOSTeamHealth: () => undefined,
    useWorkOSEnvironmentHealth: () => ({ data: undefined, error: undefined }),
    useDisconnectWorkOSTeam: (_teamId?: string) => async () => undefined,
    useInviteWorkOSTeamMember: () => async () => undefined,
    useWorkOSInvitationEligibleEmails: () => undefined,
    useAvailableWorkOSTeamEmails: () => undefined,
    useProvisionWorkOSTeam: (_teamId?: string) => async () => undefined,
    useProvisionWorkOSEnvironment: (_deploymentName?: string) => async () =>
      undefined,
    useDeleteWorkOSEnvironment: (_deploymentName?: string) => async () =>
      undefined,
    useProjectWorkOSEnvironments: () => undefined,
    useGetProjectWorkOSEnvironment: () => undefined,
    useCheckProjectEnvironmentHealth: () => async () => null,
    useProvisionProjectWorkOSEnvironment: () => async () => ({
      workosEnvironmentId: "",
      workosEnvironmentName: "",
      workosClientId: "",
      workosApiKey: "",
      newlyProvisioned: true,
      userEnvironmentName: "",
    }),
    useDeleteProjectWorkOSEnvironment: () => async () => ({
      workosEnvironmentId: "",
      workosEnvironmentName: "",
      workosTeamId: "",
    }),
  },
  useCurrentDeployment: () => ({
    id: 0,
    name: "local",
    deploymentType: "prod",
    projectId: 0,
    kind: "local",
    previewIdentifier: null,
  }),
  useIsProtectedDeployment: () => false,
  useHasProjectAdminPermissions: () => true,
  useIsDeploymentPaused: () => false,
  useProjectEnvironmentVariables: () => ({ configs: [] }),
  CloudImport: ({ sourceCloudBackupId }: { sourceCloudBackupId: number }) => (
    <div>{sourceCloudBackupId}</div>
  ),
  ErrorBoundary: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
  TeamMemberLink: ({ name }: { name: string }) => (
    <span className="font-semibold">{name}</span>
  ),
  DisconnectOverlay: () => <div>Disconnected</div>,
  useTeamUsageState: () => "Default",
  teamsURI: "",
  projectsURI: "",
  deploymentsURI: "",
  isSelfHosted: true,
  workosIntegrationEnabled: false,
  connectionStateCheckIntervalMs: 2500,
};
