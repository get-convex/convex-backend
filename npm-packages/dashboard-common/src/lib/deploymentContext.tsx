import { ConvexProvider, ConvexReactClient } from "convex/react";
import { ConnectionState, ConvexHttpClient } from "convex/browser";
import {
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useState,
} from "react";
import { useRouter } from "next/router";
import { cn } from "@ui/cn";
import { LoadingLogo } from "@ui/Loading";
import { ProjectEnvVarConfig } from "@common/features/settings/lib/types";
import { PlatformDeploymentResponse } from "@convex-dev/platform/managementApi";

export const PROVISION_PROD_PAGE_NAME = "production";
export const PROVISION_DEV_PAGE_NAME = "development";

type FallbackRender = (errorData: {
  error: Error;
  componentStack: string;
  eventId: string;
  resetError(): void;
}) => React.ReactElement;

export type DeploymentInfo = (
  | {
      ok: true;
      deploymentUrl: string;
      adminKey: string;
    }
  | { ok: false; errorCode: string; errorMessage: string }
) & {
  addBreadcrumb: (breadcrumb: {
    message?: string;
    data?: {
      [key: string]: any;
    };
  }) => void;
  captureMessage: (
    msg: string,
    severity: "fatal" | "error" | "warning" | "log" | "debug" | "info",
  ) => void;
  captureException: (e: any) => void;
  reportHttpError: (
    method: string,
    url: string,
    error: { code: string; message: string },
  ) => void;
  useCurrentTeam():
    | {
        id: number;
        name: string;
        slug: string;
      }
    | undefined;
  useTeamMembers(
    teamId?: number,
  ): { id: number; name?: string | null; email?: string }[] | undefined;
  useTeamEntitlements(teamId?: number):
    | {
        auditLogRetentionDays?: number;
        logStreamingEnabled?: boolean;
        streamingExportEnabled?: boolean;
      }
    | undefined;
  useTeamUsageState(teamId: number | null): string | undefined;
  useCurrentUsageBanner(teamId: number | null): string | null;
  useCurrentProject():
    | {
        id: number;
        name: string;
        slug: string;
      }
    | undefined;
  useCurrentDeployment(): PlatformDeploymentResponse | undefined;
  /**
   * Whether the current deployment should be treated as a "protected deployment"
   * in the dashboard UI.
   *
   * A protected deployment enables safeguards against accidental edits (for
   * example: extra confirmation dialogs, "unlock" flows before running mutations,
   * and other protections for destructive actions).
   *
   * In the Convex Cloud dashboard this is currently true for production
   * deployments. In the self-hosted dashboard this always returns false.
   */
  useIsProtectedDeployment(): boolean;
  useProjectEnvironmentVariables(
    projectId?: number,
    refreshInterval?: number,
  ): { configs: ProjectEnvVarConfig[] } | undefined;
  useHasProjectAdminPermissions(projectId: number | undefined): boolean;
  /**
   * Check whether the current admin key is allowed to perform a specific
   * deployment operation (e.g. "ViewData", "WriteData").
   *
   * Returns `true` when all operations are allowed (full admin key) or when
   * the operation is in the key's allowed list.
   */
  useIsOperationAllowed(operation: string): boolean;
  useIsDeploymentPaused(): boolean | undefined;
  useLogDeploymentEvent(): (msg: string, props?: object | null) => void;
  workOSOperations: {
    useDeploymentWorkOSEnvironment(deploymentName?: string): {
      data?:
        | {
            teamId: number;
            environment?:
              | {
                  deploymentName: string;
                  workosEnvironmentId: string;
                  workosEnvironmentName: string;
                  workosClientId: string;
                  workosTeamId: string;
                  isProduction: boolean;
                }
              | undefined
              | null;
            workosTeam?:
              | {
                  convexTeamId: number;
                  workosTeamId: string;
                  workosTeamName: string;
                  workosAdminEmail: string;
                  creatorMemberId: number;
                }
              | undefined
              | null;
          }
        | undefined;
      error?: any;
    };
    useTeamWorkOSIntegration(teamId?: string):
      | {
          teamAssociation?:
            | {
                workosTeamId: string;
                workosTeamName: string;
                adminEmail: string;
                creatorName?: string | null;
                creatorEmail: string;
              }
            | undefined
            | null;
          environments: Array<{
            deploymentName: string;
            workosEnvironmentId: string;
            workosEnvironmentName: string;
            workosClientId: string;
          }>;
        }
      | undefined;
    useWorkOSTeamHealth(teamId?: string):
      | {
          data?:
            | {
                teamProvisioned: boolean;
                teamInfo?:
                  | {
                      id: string;
                      name: string;
                      productionState:
                        | "active"
                        | "inactive"
                        | "suspended"
                        | "deleting";
                    }
                  | null
                  | undefined;
              }
            | undefined;
          error?: any;
        }
      | undefined;
    useWorkOSEnvironmentHealth(deploymentName?: string): {
      data?:
        | {
            id: string;
            name: string;
            clientId: string;
          }
        | undefined;
      error?: any;
    };
    useDisconnectWorkOSTeam(teamId?: string): (body: {
      teamId: number;
    }) => Promise<
      | {
          workosTeamId: string;
          workosTeamName: string;
        }
      | undefined
    >;
    useInviteWorkOSTeamMember(): (body: {
      teamId: number;
      email: string;
    }) => Promise<
      | {
          email: string;
          roleSlug: string;
        }
      | undefined
    >;
    useWorkOSInvitationEligibleEmails(teamId?: string):
      | {
          eligibleEmails: string[];
          adminEmail?: string | null;
        }
      | undefined;
    useAvailableWorkOSTeamEmails():
      | {
          availableEmails: string[];
          usedEmails: string[];
        }
      | undefined;
    useProvisionWorkOSTeam(teamId?: string): (body: {
      teamId: number;
      email: string;
    }) => Promise<
      | {
          workosTeamId: string;
          workosTeamName: string;
          adminEmail: string;
        }
      | undefined
    >;
    useProvisionWorkOSEnvironment(
      deploymentName?: string,
    ): (body: {
      deploymentName: string;
      isProduction: boolean;
    }) => Promise<any>;
    useDeleteWorkOSEnvironment(
      deploymentName?: string,
    ): (body: { deploymentName: string }) => Promise<any>;
    useProjectWorkOSEnvironments(projectId?: number):
      | Array<{
          workosEnvironmentId: string;
          workosEnvironmentName: string;
          workosClientId: string;
          userEnvironmentName: string;
          isProduction: boolean;
        }>
      | undefined;
    useGetProjectWorkOSEnvironment(
      projectId?: number,
      clientId?: string,
    ):
      | {
          workosEnvironmentId: string;
          workosEnvironmentName: string;
          workosClientId: string;
          workosApiKey: string;
          userEnvironmentName: string;
          isProduction: boolean;
        }
      | undefined;
    useCheckProjectEnvironmentHealth(
      projectId?: number,
      clientId?: string,
    ): () => Promise<{ id: string; name: string; clientId: string } | null>;
    useProvisionProjectWorkOSEnvironment(projectId?: number): (body: {
      environmentName: string;
    }) => Promise<{
      workosEnvironmentId: string;
      workosEnvironmentName: string;
      workosClientId: string;
      workosApiKey: string;
      newlyProvisioned: boolean;
      userEnvironmentName: string;
    }>;
    useDeleteProjectWorkOSEnvironment(projectId?: number): (
      clientId: string,
    ) => Promise<{
      workosEnvironmentId: string;
      workosEnvironmentName: string;
      workosTeamId: string;
    }>;
  };
  CloudImport(props: { sourceCloudBackupId: number }): JSX.Element;
  TeamMemberLink(props: {
    memberId?: number | null;
    name: string;
  }): JSX.Element;
  Link(props: {
    href: string;
    className?: string;
    target?: string;
    rel?: string;
    children?: ReactNode;
  }): ReactNode;
  ErrorBoundary(props: {
    children: ReactNode;
    fallback?: FallbackRender;
  }): JSX.Element;
  DisconnectOverlay(props: {
    deployment: ConnectedDeployment;
    deploymentName: string;
  }): JSX.Element;
  teamsURI: string;
  projectsURI: string;
  deploymentsURI: string;
  isSelfHosted: boolean;
  workosIntegrationEnabled: boolean;
  connectionStateCheckIntervalMs: number;
  showScheduledJobArgsInComponents: boolean;
};

export const DeploymentInfoContext = createContext<DeploymentInfo>(
  undefined as unknown as DeploymentInfo,
);

export type ConnectedDeployment = {
  client: ConvexReactClient;
  httpClient: ConvexHttpClient;
  deploymentUrl: string;
  adminKey: string;
  deploymentName: string;
};

type MaybeConnectedDeployment = {
  deployment?: ConnectedDeployment;
  deploymentName?: string;
  loading: boolean;
  errorKind: "None" | "DoesNotExist" | "NotConnected";
};

export const ConnectedDeploymentContext = createContext<{
  deployment: ConnectedDeployment;
  isDisconnected: boolean;
}>(
  // use a bad default value to detect being used incorrectly
  undefined as unknown as {
    deployment: ConnectedDeployment;
    isDisconnected: boolean;
  },
);

const MaybeConnectedDeploymentContext = createContext<MaybeConnectedDeployment>(
  // use a bad default value to detect being used incorrectly
  undefined as unknown as {
    deployment: undefined;
    loading: false;
    errorKind: "DoesNotExist";
  },
);

const useConnectedDeployment = (
  deploymentName: string | undefined,
):
  | { deployment: ConnectedDeployment; ok: true }
  | {
      ok: false;
      errorCode: string;
      errorMessage: string;
      deployment: undefined;
    }
  | undefined => {
  // Use a single setState to batch updates.
  const [state, setState] = useState<
    | {
        ok: true;
        deployment: {
          client: ConvexReactClient;
          adminKey: string;
          deploymentUrl: string;
          deploymentName: string;
        };
      }
    | {
        ok: false;
        errorCode: string;
        errorMessage: string;
        deployment: undefined;
      }
  >();

  const data = useContext(DeploymentInfoContext);

  useEffect(() => {
    if (
      deploymentName === undefined ||
      // TODO(ari): Refactor out of dashboard-common. This is only used in the cloud dashboard.
      deploymentName === PROVISION_PROD_PAGE_NAME ||
      deploymentName === PROVISION_DEV_PAGE_NAME
    )
      return;

    setState(undefined);

    let canceled = false;
    let client: ConvexReactClient;
    const getClient = async () => {
      if (canceled) return;
      if (data === undefined) {
        return;
      }
      if (!data.ok) {
        setState({
          ok: false,
          errorCode: data.errorCode,
          errorMessage: data.errorMessage,
          deployment: undefined,
        });
        return;
      }
      const { deploymentUrl, adminKey } = data;

      client = new ConvexReactClient(deploymentUrl, {
        reportDebugInfoToConvex: true,
      });
      // An internal-only API
      client.setAdminAuth(adminKey);
      setState({
        ok: true,
        deployment: { client, adminKey, deploymentUrl, deploymentName },
      });
    };
    void getClient();

    return () => {
      canceled = true;
      setState((prev) => {
        if (prev?.deployment?.client) {
          void prev.deployment.client.close();
        }
        return undefined;
      });
    };
  }, [data, deploymentName]);

  return useMemo(() => {
    if (!state) return undefined;
    if (state.ok) {
      const {
        deployment: { deploymentUrl },
      } = state;
      return {
        ok: true,
        deployment: {
          httpClient: new ConvexHttpClient(deploymentUrl),
          ...state.deployment,
        },
      };
    }
    return state;
  }, [state]);
};

export type DeploymentApiProviderProps = {
  children: React.ReactNode;
  deploymentOverride?: string;
};

// A silly, standard hack to dodge warnings about useLayoutEffect on the server.
const useIsomorphicLayoutEffect =
  typeof window !== "undefined" ? useLayoutEffect : useEffect;

export function DeploymentApiProvider({
  children,
  deploymentOverride,
}: DeploymentApiProviderProps) {
  // Router is not available when this component runs on pages during static generation
  // and hydration from static generation, so router can only be accessed in useEffects.
  const router = useRouter();
  const [deploymentName, setDeploymentName] = useState<string | undefined>(
    deploymentOverride,
  );
  useIsomorphicLayoutEffect(() => {
    if (deploymentOverride) {
      return;
    }
    if (router.isReady && typeof router.query?.deploymentName === "string") {
      setDeploymentName(router.query?.deploymentName);
    } else {
      setDeploymentName(undefined);
    }
  }, [router.isReady, router.query, deploymentOverride]);

  const deploymentInfoContext = useContext(DeploymentInfoContext);

  const connected = useConnectedDeployment(deploymentName);
  // eslint-disable-next-line react/jsx-no-constructed-context-values
  let value: MaybeConnectedDeployment = {
    deployment: undefined,
    deploymentName,
    loading: true,
    errorKind: "None",
  };
  if (connected?.ok) {
    value = {
      deployment: connected.deployment,
      deploymentName,
      loading: false,
      errorKind: "None",
    };
  } else if (
    connected?.errorCode === "InstanceNotFound" ||
    connected?.errorCode === "DeploymentNotFound"
  ) {
    value = {
      deployment: undefined,
      deploymentName,
      loading: false,
      errorKind: "DoesNotExist",
    };
  } else if (connected && !connected?.ok) {
    deploymentInfoContext?.captureMessage(
      `Can't connect to deployment ${connected?.errorCode} ${connected?.errorMessage}`,
      "warning",
    );
  }

  return (
    <MaybeConnectedDeploymentContext.Provider value={value}>
      {children}
    </MaybeConnectedDeploymentContext.Provider>
  );
}

export function WaitForDeploymentApi({
  children,
  sizeClass,
}: {
  children: ReactNode;
  sizeClass?: string;
}) {
  const connected = useContext(MaybeConnectedDeploymentContext);
  const router = useRouter();
  if (connected === undefined) {
    throw new Error(
      "WaitForDeploymentApi used outside of DeploymentApiProvider",
    );
  }

  const { deployment, loading, errorKind } = connected;
  if (errorKind === "DoesNotExist") {
    void router.push("/404?reason=deployment_not_found");
    return null;
  }
  if (loading) {
    return (
      <div
        className={cn(
          "flex h-full w-full items-center justify-center",
          sizeClass,
        )}
      >
        <LoadingLogo />
      </div>
    );
  }
  // we don't know that it's defined, but if not it's a programming error
  const { client } = deployment!;

  return (
    <ConvexProvider client={client}>
      <DeploymentWithConnectionState deployment={deployment!}>
        {children}
      </DeploymentWithConnectionState>
    </ConvexProvider>
  );
}

function DeploymentWithConnectionState({
  deployment,
  children,
}: {
  deployment: ConnectedDeployment;
  children: ReactNode;
}) {
  const {
    captureMessage,
    addBreadcrumb,
    DisconnectOverlay,
    connectionStateCheckIntervalMs,
  } = useContext(DeploymentInfoContext);
  const { client, deploymentUrl, deploymentName } = deployment;
  const [lastObservedConnectionState, setLastObservedConnectionState] =
    useState<
      | {
          state: ConnectionState;
          time: Date;
        }
      | "LocalDeploymentMismatch"
      | null
    >(null);
  const [isDisconnected, setIsDisconnected] = useState<boolean | null>(null);

  const handleConnectionStateChange = useCallback(
    async (
      state: ConnectionState,
      previousState: {
        time: Date;
        state: ConnectionState;
      } | null,
    ): Promise<
      "Unknown" | "Disconnected" | "Connected" | "LocalDeploymentMismatch"
    > => {
      if (previousState === null) {
        return "Unknown";
      }
      if (
        previousState.time.getTime() <
        Date.now() - connectionStateCheckIntervalMs * 2
      ) {
        // If the previous state was observed a while ago, consider it stale (maybe the tab
        // got backgrounded).
        return "Unknown";
      }

      if (state.isWebSocketConnected === false) {
        if (previousState.state.isWebSocketConnected === false) {
          // we've been in state `Disconnected` twice in a row, consider the deployment
          // to be disconnected.
          return "Disconnected";
        }
        return "Unknown";
      }
      if (state.isWebSocketConnected === true) {
        // If this is a local deployment, check that the instance name matches what we expect.
        if (deploymentName.startsWith("local-")) {
          let instanceNameResp: Response | null = null;
          try {
            instanceNameResp = await fetch(
              new URL("/instance_name", deploymentUrl),
            );
          } catch {
            // do nothing, we'll check the WS connection status below
          }
          if (instanceNameResp !== null && instanceNameResp.ok) {
            const instanceName = await instanceNameResp.text();
            if (instanceName !== deploymentName) {
              return "LocalDeploymentMismatch";
            }
          }
        }
        return "Connected";
      }
      return "Unknown";
    },
    [deploymentName, deploymentUrl, connectionStateCheckIntervalMs],
  );

  useEffect(() => {
    // Poll `.connectionState()`. If we're disconnected twice in a row,
    // consider the deployment to be disconnected.
    const checkConnection = setInterval(async () => {
      if (lastObservedConnectionState === "LocalDeploymentMismatch") {
        // Connection status doesn't matter since we're connected to the wrong deployment
        return;
      }
      // Check WS connection status -- if we're disconnected twice in a row, treat
      // the deployment as disconnected.
      const nextConnectionState = client.connectionState();
      const isLocalDeployment = deploymentName.startsWith("local-");
      const result = await handleConnectionStateChange(
        nextConnectionState,
        lastObservedConnectionState,
      );
      setLastObservedConnectionState({
        state: nextConnectionState,
        time: new Date(),
      });
      switch (result) {
        case "Disconnected":
          // If this is first time transitioning to disconnected, log to sentry that we've disconnected
          if (isDisconnected !== true) {
            if (!isLocalDeployment) {
              addBreadcrumb({
                message: `Cloud deployment disconnected: ${deploymentName}`,
                data: {
                  hasEverConnected: nextConnectionState.hasEverConnected,
                  connectionCount: nextConnectionState.connectionCount,
                  connectionRetries: nextConnectionState.connectionRetries,
                },
              });
              // Log to sentry including the instance name when we seem to be unable to connect to a cloud deployment
              captureMessage(`Cloud deployment is disconnected`, "warning");
            }
          }
          setIsDisconnected(true);
          break;
        case "LocalDeploymentMismatch":
          setLastObservedConnectionState("LocalDeploymentMismatch");
          break;
        case "Unknown":
          setIsDisconnected(null);
          break;
        case "Connected":
          // If transitioning from disconnected to connected, log to sentry that we've reconnected
          if (isDisconnected === true) {
            if (!isLocalDeployment) {
              addBreadcrumb({
                message: `Cloud deployment reconnected: ${deploymentName}`,
              });
              // Log to sentry including the instance name when we seem to be unable to connect to a cloud deployment
              captureMessage(`Cloud deployment has reconnected`, "warning");
            }
          }
          setIsDisconnected(false);
          break;
        default: {
          result satisfies never;
          throw new Error(`Unknown connection state: ${result}`);
        }
      }
    }, connectionStateCheckIntervalMs);
    return () => clearInterval(checkConnection);
  }, [
    lastObservedConnectionState,
    deploymentName,
    deploymentUrl,
    client,
    addBreadcrumb,
    captureMessage,
    handleConnectionStateChange,
    isDisconnected,
    connectionStateCheckIntervalMs,
  ]);
  const value = useMemo(
    () => ({
      deployment,
      isDisconnected: isDisconnected === true,
    }),
    [deployment, isDisconnected],
  );
  return (
    <>
      {isDisconnected && (
        <DisconnectOverlay
          deployment={deployment}
          deploymentName={deploymentName}
        />
      )}
      <ConnectedDeploymentContext.Provider value={value}>
        {children}
      </ConnectedDeploymentContext.Provider>
    </>
  );
}
