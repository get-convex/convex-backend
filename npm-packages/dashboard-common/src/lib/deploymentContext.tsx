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
import { Button } from "@ui/Button";
import {
  CheckCircledIcon,
  CrossCircledIcon,
  ExternalLinkIcon,
  InfoCircledIcon,
  LinkBreak2Icon,
} from "@radix-ui/react-icons";
import { Sheet } from "@ui/Sheet";
import { Spinner } from "@ui/Spinner";
import { Tooltip } from "@ui/Tooltip";
import { Callout } from "@ui/Callout";

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
  useCurrentDeployment():
    | {
        id: number;
        name: string;
        projectId: number;
        deploymentType: "prod" | "dev" | "preview" | "custom";
        kind: "local" | "cloud";
        previewIdentifier?: string | null;
      }
    | undefined;
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
  useIsDeploymentPaused(): boolean | undefined;
  useLogDeploymentEvent(): (msg: string, props?: object | null) => void;
  workOSOperations: {
    useDeploymentWorkOSEnvironment(deploymentName?: string):
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
                      productionState: "active" | "inactive";
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

function useIsSafari(): boolean {
  const [isSafari, setIsSafari] = useState(false);
  useEffect(() => {
    setIsSafari(
      // https://stackoverflow.com/a/23522755
      /^((?!chrome|android).)*safari/i.test(navigator.userAgent),
    );
  }, []);
  return isSafari;
}

function useIsBrave(): boolean {
  const [isBrave, setIsBrave] = useState(false);
  useEffect(() => {
    setIsBrave("brave" in navigator);
  }, []);
  return isBrave;
}

function DisconnectedOverlay({ children }: { children: ReactNode }) {
  return (
    <div className="absolute z-50 mt-[3.5rem] flex h-[calc(100vh-3.5rem)] w-full items-center justify-center backdrop-blur-[4px]">
      <Sheet className="scrollbar flex max-h-[80vh] max-w-[28rem] animate-fadeInFromLoading flex-col items-start gap-2 overflow-y-auto rounded-xl bg-background-secondary/90 backdrop-blur-[8px]">
        <h3 className="mb-4 flex items-center gap-3">
          <div className="flex aspect-square h-[2.625rem] shrink-0 items-center justify-center rounded-lg border bg-gradient-to-tr from-yellow-200 to-util-brand-yellow text-yellow-900 shadow-md">
            <LinkBreak2Icon className="size-6" />
          </div>
          Connection Issue
        </h3>
        {children}
      </Sheet>
    </div>
  );
}

export function LocalDeploymentDisconnectOverlay() {
  const isSafari = useIsSafari();
  const isBrave = useIsBrave();

  return (
    <DisconnectedOverlay>
      {isSafari ? (
        <>
          <p className="mb-1">Safari blocks connections to localhost.</p>
          <p className="mb-4">
            We recommend using another browser when using local deployments.
          </p>
          <Button
            href="https://docs.convex.dev/cli/local-deployments#safari"
            variant="neutral"
            icon={<ExternalLinkIcon />}
            target="_blank"
          >
            Learn more
          </Button>
        </>
      ) : isBrave ? (
        <>
          <p className="mb-2">
            Brave blocks connections to localhost by default. We recommend using
            another browser or{" "}
            <a
              href="https://docs.convex.dev/cli/local-deployments#brave"
              target="_blank"
              rel="noreferrer"
              className="text-content-link hover:underline"
            >
              setting up Brave to allow localhost connections
            </a>
            .
          </p>
          <Button
            href="https://docs.convex.dev/cli/local-deployments#brave"
            variant="neutral"
            icon={<ExternalLinkIcon />}
            target="_blank"
          >
            Learn more
          </Button>
        </>
      ) : (
        <>
          <p className="mb-2">
            Check that <code className="text-sm">npx convex dev</code> is
            running successfully.
          </p>
          <p>
            If you have multiple devices you use with this Convex project, the
            local deployment may be running on a different device, and can only
            be accessed on that machine.
          </p>
        </>
      )}
    </DisconnectedOverlay>
  );
}

export function SelfHostedDisconnectOverlay() {
  const deploymentInfo = useContext(DeploymentInfoContext);
  const deploymentUrl = deploymentInfo.ok ? deploymentInfo.deploymentUrl : "";
  return (
    <DisconnectedOverlay>
      <p className="mb-2">
        Check that your Convex server is running and accessible at{" "}
        <code className="text-sm">{deploymentUrl}</code>.
      </p>
      <p>If you continue to have issues, try restarting your Convex server.</p>
    </DisconnectedOverlay>
  );
}

function useCanReachDeploymentOverHTTP(deploymentUrl: string): boolean | null {
  const [isReachable, setIsReachable] = useState<boolean | null>(null);

  useEffect(() => {
    let canceled = false;

    const checkReachability = async () => {
      try {
        await fetch(deploymentUrl, {
          method: "HEAD",
          mode: "no-cors",
        });
        if (!canceled) {
          setIsReachable(true);
        }
      } catch {
        if (!canceled) {
          setIsReachable(false);
        }
      }
    };

    void checkReachability();

    return () => {
      canceled = true;
    };
  }, [deploymentUrl]);

  return isReachable;
}

export function CloudDisconnectOverlay({
  deployment,
  deploymentName,
  openSupportForm,
  statusWidget,
}: {
  deployment: ConnectedDeployment;
  deploymentName: string;
  openSupportForm?: (defaultSubject: string, defaultMessage: string) => void;
  statusWidget?: React.ReactNode;
}) {
  const isReachable = useCanReachDeploymentOverHTTP(deployment.deploymentUrl);

  const handleContactSupport = useCallback(() => {
    const defaultMessage = `I'm unable to connect to my deployment "${deploymentName}".

Deployment URL: ${deployment.deploymentUrl}
HTTP reachable: ${isReachable === null ? "checking..." : isReachable ? "yes" : "no"}
Browser Version: ${navigator.userAgent}

Please help me troubleshoot this connection issue.`;

    const defaultSubject = `Unable to connect to ${deploymentName}`;

    if (openSupportForm) {
      openSupportForm(defaultSubject, defaultMessage);
    }
  }, [deploymentName, deployment.deploymentUrl, isReachable, openSupportForm]);

  return (
    <DisconnectedOverlay>
      <div className="space-y-4">
        <div>
          <h4 className="mb-2">Connection Status</h4>
          <div className="flex flex-col gap-2">
            <p className="flex items-center gap-1 text-sm">
              <div className="w-fit rounded-full bg-background-error p-1">
                <CrossCircledIcon
                  className="text-content-error"
                  aria-hidden="true"
                />
              </div>
              WebSocket connection failed
            </p>
            {isReachable === null ? (
              <p className="flex items-center gap-1 text-sm text-content-secondary">
                <div className="p-1">
                  <Spinner />
                </div>
                Checking HTTP connection...
              </p>
            ) : isReachable ? (
              <p className="flex items-center gap-1 text-sm">
                <div className="w-fit rounded-full bg-background-success p-1">
                  <CheckCircledIcon
                    className="text-content-success"
                    aria-hidden="true"
                  />
                </div>
                HTTP connection successful
              </p>
            ) : (
              <p className="flex items-center gap-1 text-sm">
                <div className="w-fit rounded-full bg-background-error p-1">
                  <CrossCircledIcon
                    className="text-content-error"
                    aria-hidden="true"
                  />
                </div>
                HTTP connection failed
              </p>
            )}
          </div>
        </div>

        <div>
          <h4 className="mb-2">Troubleshooting</h4>
          {isReachable ? (
            <>
              <Callout className="mb-3" variant="hint">
                <div className="flex flex-col gap-2">
                  <h5 className="flex items-center gap-1">
                    <InfoCircledIcon />
                    Your deployment is online
                  </h5>
                  <p>
                    This connection issue is likely due to a problem with your
                    browser or network connection.
                  </p>
                </div>
              </Callout>
              <p className="mb-2">
                Please try the following troubleshooting steps:
              </p>
            </>
          ) : (
            <p className="mb-2 text-sm">
              There may be a client-side network issue. Try:
            </p>
          )}
          <ul className="ml-2 list-inside list-disc space-y-1 text-sm">
            <li>
              Switching to a different network. (i.e. WiFi, ethernet, or
              cellular)
            </li>
            <li>
              <span className="inline-flex items-center gap-1">
                Reloading the browser page
                <Tooltip tip="The Convex dashboard will automatically attempt to reconnect to your deployment, but refreshing the page may help in some cases.">
                  <InfoCircledIcon className="shrink-0" />
                </Tooltip>
              </span>
            </li>
            <li>Disabling your VPN</li>
            <li>Disabling browser extensions</li>
          </ul>
        </div>

        {statusWidget && (
          <div>
            <h4 className="mb-2">Convex Status</h4>
            {statusWidget}
          </div>
        )}

        {isReachable === false && openSupportForm && (
          <div className="border-t pt-2">
            <p className="text-sm text-content-secondary">
              <Button inline onClick={handleContactSupport}>
                Tried all of the troubleshooting steps? Contact support
              </Button>
            </p>
          </div>
        )}
      </div>
    </DisconnectedOverlay>
  );
}
