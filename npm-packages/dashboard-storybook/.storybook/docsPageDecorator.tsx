import { DeploymentInfoProvider } from "../../dashboard/src/providers/DeploymentInfoProvider";
import { ReactNode, useContext, useEffect } from "react";
import { DecoratorFunction } from "storybook/internal/types";
import { ReactRenderer } from "@storybook/nextjs";
import { mocked, fn } from "storybook/test";
import type { User } from "@workos-inc/node";
import { DashboardHeader } from "../../dashboard/src/components/header/DashboardHeader";
import { useAccessToken } from "../../dashboard/src/hooks/useServerSideData";
import {
  useCurrentTeam,
  useTeamEntitlements,
  useTeamMembers,
  useTeams,
} from "../../dashboard/src/api/teams";
import {
  useProjectBySlug,
  useCurrentProject,
  usePaginatedProjects,
  useProjectById,
} from "../../dashboard/src/api/projects";
import {
  useIsCurrentMemberTeamAdmin,
  useHasProjectAdminPermissions,
} from "../../dashboard/src/api/roles";
import { useHasOptedIn } from "../../dashboard/src/api/optins";
import {
  flagDefaults,
  useLaunchDarkly,
} from "../../dashboard/src/hooks/useLaunchDarkly";
import { useProfile } from "../../dashboard/src/api/profile";
import { AuthContext } from "../../dashboard/src/providers/AuthProvider/AuthContext";
import {
  useTeamOrbSubscription,
  useListInvoices,
  useGetSpendingLimits,
  useListPlans,
} from "../../dashboard/src/api/billing";
import {
  useDeployments,
  useCurrentDeployment,
  useDeploymentByName,
  useDeploymentRegions,
} from "../../dashboard/src/api/deployments";
import { deploymentAuth } from "../../dashboard/src/lib/deploymentAuth";
import { useTeamUsageState } from "../../dashboard/src/api/usage";
import { useReferralState } from "../../dashboard/src/api/referrals";
import { usePostHog } from "../../dashboard/src/hooks/usePostHog";
import { useListCloudBackups } from "../../dashboard/src/api/backups";
import {
  useCreateTeamAccessToken,
  useDeployKeys,
  useDeleteDeployKey,
} from "../../dashboard/src/api/accessTokens";
import {
  useCreateVanityDomain,
  useDeleteVanityDomain,
  useListVanityDomains,
} from "../../dashboard/src/api/vanityDomains";
import { LocalDevCallout } from "../../dashboard-common/src/elements/LocalDevCallout";
import { PlatformDeploymentResponse } from "@convex-dev/platform/managementApi";
import { CloudDisconnectOverlay } from "../../dashboard-common/src/features/disconnectOverlay/CloudDisconnectOverlay";
import { DeploymentProvider } from "../../dashboard/src/components/projectSettings/CustomDomains";
import { mockConvexReactClient } from "../../dashboard-common/src/lib/mockConvexReactClient";
import udfs from "../../dashboard-common/src/udfs";
import { ConvexProvider } from "convex/react";
import { DeploymentDashboardLayout } from "../../dashboard-common/src/layouts/DeploymentDashboardLayout";
import {
  ConnectedDeploymentContext,
  DeploymentInfoContext,
} from "../../dashboard-common/src/lib/deploymentContext";
import { useTableShapes } from "../../dashboard-common/src/lib/deploymentApi";

const MOCK_PROFILE_PICTURE_URL =
  "data:image/webp;base64,UklGRqYFAABXRUJQVlA4IJoFAACwFwCdASpAAEAAPpE8mUiloyIhLBqqaLASCWwAnTLLxB/G+ZjZ2vaGntzejrbgc7TpzPoAT6/2j/VfBXxxe2cz3GXaB9XkZTJNa93iEAHVd6qaw5QA9wD5YNDD1inUcsafQyKkL3va9r5b6tHRyOzYkfx3XdT7BBLNpPvVQDroRE+86vqS4IbQ/TuVbxR22x/qQ3ATiNd6VDuwkyEDi67ee40Djosp8rgLpxKRgzL15tn235yg5nJiZkZLMqLeVsk+AqZEJiAA/ub/+ti932/SJ7Ed6JWfB5D7z9NBn34kJK3wWGn17qnGuhyXm3arHw5zYWWoqoTVlBBWsym+VLk6FBA3KCPlX9hyu9YNhcCear7jv4Onz3JXFtmd7O7nog1WPOQgejCX4W5PJ6mLS19O3AZfSyn/rGPbQOBZuqsw51VzDBoJXR8K6rio+Qt3X1nLwwdc5Yr8Ki/duH8c9/F6rcRW30/0I8fHTkJ0IY53taSJfuiaGJ5VsghdyUG3p7mkvD/HMN+467NVP26wO3v+cAd5ke9N8pM0FyM4moFWtHw3FSUH/jzn4+38UZuYpNYI1v8Noumcs/sfQ/IEhE76bFQMsGvAgUCsd89hWG2w+g1U0Nd4Ugiv0deXMH28+lq6JHYXKBsj9uy6LLvK4sx/yXtsYfF3uj8z8bGpcWLNdZ3hWNmYZH7dTCzFm6P/zRcH0Pd0VNZ8rRc2/+Pq6f5Hq5qHwywa8Dq6FL9dr5xmIowIfzxyKWEITu6bfUBOdaloCxcD6+NhKroqeAL2dLq1qW58eXyrZFVb1adugT9qIC+YlnbzZMyR3d5L7vezSng58Pd4scxsgu7Koi5/PE1MS3+JD1vmLd+0eQwd5Shk1pSTkwI7raQgwPAtF0br9RaLsW+p7wPb1fY51wj++AyVK/i5dlmaktv7o/T49TCUfqLRPm796NCM16jK/tViNOlBonU7rLcOvq0S2wXjm9KhUFJmOrwM5c8l+LmdFEOFZQdFzRg4bgxZPe380SUQev6O/kSjPKz/yqt4QFFvkvOQhWc1o4Yk98XA1XOQiPwzr3eouH/uQaGw4WOF86+dKXsk0rPhzqh/ThVbS9T/SQUKj1HbQ8LHwUt3RRXju1cZK6V3CmfO8Ep0abVycofCiBlgse12mpsP+/yhR0jtAnjjaq5D1P7bQq8Suck/kJWBEU2MpwqU3JZ62GEDddHiKi/D7Z5jcvBzSGTF1wmlEbl1MAF8rfPq7ncROFLF1busN73evw9mEUN+kIS1WuWxOGz66zIZS+gF098q9kXHq7xhV2bsPemqpqHJ/FAkDqOM1AS6Mg+3MJVD/JsDWETX5nUO6W1I1Nts35WcLI4wUtbtKvfp+1XbB+h9FP1OZ8DeGiMX+iL+KzLUPzNbrYb0+4ZGXOmhl0HYdAvWzd6Zd7JssNGNKVOgvYcqi6ljy/JYLnk0JgePXqg2jEpZuYRG8Zz66yg/NOPHn96zN4EeppQtzso6paGyrrGJ3fR+eh6+y2kV/vXInPU3dw7mbC/8gQUnTFoHg2RrQCCRypP3HO0LlG8uT47z+yFSx3lDXFs/iPETPXP9RhDYkg2Ja7XozibpaLMr3uNfNzb9Z8YuCEeB5OBrg19n92+08SgaHjVK/m+1jjv1s4vAhJCMJOZRUS0VZXTMD2kRQTPcgYosp7Tt/uo0i3yX4zzb25rvbeR+smvjoMuqG6MrXqZ2w0eU22DOdWSd3kuKUfYxZvLEuWwqtVKawcX5yyUHo7wryoMcnJzF/iqh1lVkmla8pBdBBISSOA8u1Ef+tvGYOf3nVscALWEwwgjIgW5Jz17pmObucgE9xNaEfwpu72nhFZOPecK8zRkTPZ00iotWPZav6MaJ3rd9bXN3uG1VzJABCQrEAFDkONmytDf6AAA=";
const mockUser: User = {
  object: "user",
  id: "user_workos123",
  email: "nicolas@acme.dev",
  emailVerified: true,
  firstName: "Nicolas",
  lastName: "Ettlin",
  profilePictureUrl: MOCK_PROFILE_PICTURE_URL,
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
  lastSignInAt: new Date().toISOString(),
  externalId: null,
  metadata: {},
};

/**
 * Stories in dashboard/src/docs/pages/ replicate dashboard pages.
 * This decorator adds some useful mocks for various hooks used
 * in the dashboard, and replicates the app-wide layout from _app.tsx.
 */
export const docsPageDecorator: DecoratorFunction<ReactRenderer> = (
  storyFn,
  context,
) => {
  const { title } = context;
  const isDocsPageStory = title.startsWith("docs/pages/");
  if (!isDocsPageStory) {
    return storyFn();
  }

  const mockTeam = {
    id: 2,
    creator: 1,
    slug: "acme",
    name: "Acme Corp",
    suspended: false,
    referralCode: "ACME01",
    referredBy: null,
  };
  const mockTeamMembers: NonNullable<ReturnType<typeof useTeamMembers>> = [
    {
      id: 1,
      name: "Nicolas Ettlin",
      email: "nicolas@acme.dev",
      role: "admin",
    },
    {
      id: 2,
      email: "ari@acme.dev",
      name: "Ari Trakh",
      role: "admin",
    },
  ];
  const mockProfile = {
    id: 1,
    name: "Nicolas Ettlin",
    email: "nicolas@acme.dev",
  };
  const mockProject = {
    id: 7,
    teamId: mockTeam.id,
    name: "My amazing app",
    slug: "my-amazing-app",
    isDemo: false,
    createTime: Date.now(),
    prodDeploymentName: "musical-otter-456",
    devDeploymentName: "happy-capybara-123",
  } as NonNullable<ReturnType<typeof useProjectBySlug>>;
  const shouldMockCurrentProject = title.startsWith("docs/pages/project/");
  const shouldMockCurrentDeployment = title.startsWith(
    "docs/pages/project/deployment/",
  );
  const mockTeamEntitlements = {
    auditLogRetentionDays: 90,
    customDomainsEnabled: true,
    deploymentClassSelectionEnabled: false,
    logStreamingEnabled: true,
    managementApiEnabled: true,
    maxChefTokens: 50_000_000,
    maxCloudBackups: 50,
    maxDeployments: 120,
    maxTeamMembers: 20,
    periodicBackupsEnabled: true,
    previewDeploymentRetentionDays: 14,
    ssoEnabled: false,
    streamingExportEnabled: true,
    teamMaxActionCompute: 250,
    teamMaxDatabaseBandwidth: 53_687_091_200,
    teamMaxDatabaseStorage: 53_687_091_200,
    teamMaxFileBandwidth: 53_687_091_200,
    teamMaxFileStorage: 107_374_182_400,
    teamMaxFunctionCalls: 25_000_000,
    teamMaxVectorBandwidth: 10_737_418_240,
    teamMaxVectorStorage: 1_073_741_824,
  };
  const mockSubscription: ReturnType<
    typeof useTeamOrbSubscription
  >["subscription"] = {
    billingContact: {
      email: "billing@acme.dev",
      name: "Acme Corporation",
    },
    billingAddress: {
      line1: "444 De Haro St",
      line2: "Suite 219",
      city: "San Francisco",
      state: "CA",
      postal_code: "94107",
      country: "US",
    },
    nextBillingPeriodStart: new Date(
      Date.now() + 30 * 24 * 60 * 60 * 1000,
    ).toISOString(),
    plan: {
      id: "CONVEX_PROFESSIONAL",
      name: "Professional",
      description: "The professional plan.",
      status: "active",
      planType: "CONVEX_PROFESSIONAL",
      seatPrice: 25,
    },
    status: "active",
  };

  mocked(useTeams).mockReturnValue({
    selectedTeamSlug: mockTeam.slug,
    teams: [mockTeam],
  });
  mocked(useTeamMembers).mockReturnValue(mockTeamMembers);
  mocked(useCurrentTeam).mockReturnValue(mockTeam);
  mocked(useIsCurrentMemberTeamAdmin).mockReturnValue(true);
  mocked(usePaginatedProjects).mockReturnValue({
    items: [mockProject],
    pagination: {
      hasMore: false,
    },
    isLoading: false,
  });
  mocked(useProfile).mockReturnValue(mockProfile);
  mocked(useTeamEntitlements).mockReturnValue(mockTeamEntitlements);
  mocked(useProjectBySlug).mockReturnValue(
    shouldMockCurrentProject ? mockProject : undefined,
  );
  mocked(useCurrentProject).mockReturnValue(
    shouldMockCurrentProject
      ? (mockProject as ReturnType<typeof useCurrentProject>)
      : undefined,
  );
  mocked(useProjectById).mockImplementation(() => ({
    project: mockProject,
    isLoading: false,
    error: undefined,
  }));
  mocked(useTeamOrbSubscription).mockReturnValue({
    isLoading: false,
    subscription: mockSubscription,
  });
  mocked(useListInvoices).mockReturnValue({
    isLoading: false,
    invoices: [],
  });
  mocked(useListPlans).mockReturnValue({
    plans: [
      {
        id: "ndRbFq64RtiuLeqy",
        name: "Convex Starter",
        description: "For personal projects and prototypes.",
        status: "active",
        seatPrice: null,
        planType: "CONVEX_STARTER_PLUS",
      },
      {
        id: "LtLxrR95Wqn5ScNJ",
        name: "Convex Professional",
        description: "For small teams working together on growing projects.",
        status: "active",
        seatPrice: 25,
        planType: "CONVEX_PROFESSIONAL",
      },
    ],
    isLoading: false,
  });
  mocked(useGetSpendingLimits).mockReturnValue({
    isLoading: false,
    spendingLimits: {
      state: null,
      disableThresholdCents: null,
      warningThresholdCents: null,
    },
  });
  mocked(useTeamUsageState).mockReturnValue("Default");
  mocked(useReferralState).mockReturnValue(null);
  const DEV_DEPLOYMENT: PlatformDeploymentResponse = {
    id: 11,
    name: "happy-capybara-123",
    deploymentType: "dev",
    kind: "cloud",
    isDefault: true,
    projectId: mockProject.id,
    creator: 1,
    createTime: Date.now(),
    class: "s256",
    deploymentUrl: "https://happy-capybara-123.convex.cloud",
    reference: "dev/nicolas",
    region: "aws-us-east-1",
  };
  mocked(useDeployments).mockReturnValue({
    deployments: [
      DEV_DEPLOYMENT,
      {
        id: 12,
        name: "musical-otter-456",
        deploymentType: "prod",
        kind: "cloud",
        isDefault: true,
        projectId: mockProject.id,
        creator: 1,
        createTime: Date.now(),
        class: "s256",
        deploymentUrl: "https://musical-otter-456.eu-west-1.convex.cloud",
        reference: "production",
        region: "aws-eu-west-1",
      },
    ],
    isLoading: false,
  });
  mocked(useHasProjectAdminPermissions).mockReturnValue(true);
  mocked(useHasOptedIn).mockReturnValue({
    hasOptedIn: true,
    isLoading: false,
    optInsWithMessageToAccept: [],
  });
  mocked(useLaunchDarkly).mockReturnValue({
    ...flagDefaults,
    enableStatuspageWidget: false,
  });
  mocked(useCurrentDeployment).mockReturnValue(
    shouldMockCurrentDeployment ? DEV_DEPLOYMENT : undefined,
  );
  mocked(deploymentAuth).mockImplementation(async (deploymentName) => {
    return {
      ok: true,
      deploymentUrl: `https://${deploymentName}.convex.cloud`,
      adminKey: "STORYBOOK-FAKE-KEY",
    };
  });
  mocked(useDeploymentByName).mockReturnValue(undefined);
  mocked(useDeploymentRegions).mockReturnValue({
    regions: [
      {
        displayName: "Europe (Ireland)",
        name: "aws-eu-west-1",
        available: true,
      },
      {
        displayName: "US East (N. Virginia)",
        name: "aws-us-east-1",
        available: true,
      },
    ],
    isLoading: false,
  });
  mocked(usePostHog).mockReturnValue({
    capture: fn(),
    posthog: undefined,
  });
  mocked(useListCloudBackups).mockReturnValue([]);
  mocked(useListVanityDomains).mockReturnValue([]);
  mocked(useCreateVanityDomain).mockReturnValue(fn());
  mocked(useDeleteVanityDomain).mockReturnValue(fn());
  mocked(useCreateTeamAccessToken).mockReturnValue(fn());
  mocked(useDeployKeys).mockReturnValue([]);
  mocked(useDeleteDeployKey).mockReturnValue(fn());
  mocked(LocalDevCallout).mockReturnValue(null);
  mocked(CloudDisconnectOverlay).mockReturnValue(null);
  mocked(useTableShapes).mockReturnValue({
    tables: new Map(),
    hadError: false,
  });

  // DeploymentProvider normally creates a real ConvexReactClient that opens a
  // WebSocket connection. Replace the real provider chain with a lightweight
  // ConvexProvider backed by a mock client so the components inside (e.g.
  // DeploymentDomainInfo) can still call useQuery without hitting a real backend.
  const mockClient = mockConvexReactClient()
    .registerQueryFake(udfs.convexCloudUrl.default, () => undefined)
    .registerQueryFake(udfs.components.list, () => [])
    .registerQueryFake(udfs.deploymentState.deploymentState, () => ({
      _id: "" as any,
      _creationTime: 0,
      state: "running" as const,
    }))
    .registerQueryFake(udfs.modules.listForAllComponents, () => [])
    .registerQueryFake(udfs.getSchemas.default, () => ({}));
  mocked(DeploymentProvider).mockImplementation(({ children }) => (
    <ConvexProvider client={mockClient}>{children}</ConvexProvider>
  ));
  const mockConnectedDeployment = {
    deployment: {
      client: mockClient,
      httpClient: {} as never,
      deploymentUrl: DEV_DEPLOYMENT.deploymentUrl,
      adminKey: "STORYBOOK-FAKE-KEY",
      deploymentName: DEV_DEPLOYMENT.name,
    },
    isDisconnected: false,
  };

  return (
    <DocsShell
      deployment={shouldMockCurrentDeployment ? DEV_DEPLOYMENT.name : null}
      mockConnectedDeployment={
        shouldMockCurrentDeployment ? mockConnectedDeployment : null
      }
      mockClient={shouldMockCurrentDeployment ? mockClient : null}
    >
      {storyFn()}
    </DocsShell>
  );
};

function DocsShell({
  children,
  deployment,
  mockConnectedDeployment,
  mockClient,
}: React.PropsWithChildren<{
  deployment: string | null;
  mockConnectedDeployment: {
    deployment: {
      client: any;
      httpClient: never;
      deploymentUrl: string;
      adminKey: string;
      deploymentName: string;
    };
    isDisconnected: boolean;
  } | null;
  mockClient: any | null;
}>) {
  const [accessToken, setAccessToken] = useAccessToken();

  useEffect(() => {
    setAccessToken("storybook-docs-token");
  }, [setAccessToken]);

  const pageContents = (
    <div className="flex h-screen flex-col">
      <DashboardHeader />
      <div className="flex-1 overflow-auto">
        {deployment && mockConnectedDeployment && mockClient ? (
          <ConnectedDeploymentContext.Provider value={mockConnectedDeployment}>
            <ConvexProvider client={mockClient}>
              <DeploymentLayoutWhenReady>{children}</DeploymentLayoutWhenReady>
            </ConvexProvider>
          </ConnectedDeploymentContext.Provider>
        ) : (
          children
        )}
      </div>
    </div>
  );

  return (
    <AuthContext.Provider
      value={{
        user: mockUser,
        isAuthenticated: true,
        isLoading: false,
        error: null,
      }}
    >
      {deployment ? (
        // Waiting for the access token to be loaded
        // because the useEffect in `<DeploymentInfoProvider />`
        // doesn’t rerun when the access token changes
        !accessToken ? null : (
          <DeploymentInfoProvider deploymentOverride={deployment}>
            {pageContents}
          </DeploymentInfoProvider>
        )
      ) : (
        pageContents
      )}
    </AuthContext.Provider>
  );
}

// DeploymentInfoProvider renders children without the context on the first
// render (before the async deploymentAuth resolves). Guard against calling
// DeploymentDashboardLayout until the context is available.
function DeploymentLayoutWhenReady({
  children,
}: React.PropsWithChildren<object>) {
  const deploymentInfo = useContext(DeploymentInfoContext);
  if (!deploymentInfo) return null;
  return (
    <DeploymentDashboardLayout>
      {children as JSX.Element}
    </DeploymentDashboardLayout>
  );
}
