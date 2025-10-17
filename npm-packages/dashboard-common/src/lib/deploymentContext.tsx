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
import { ExternalLinkIcon } from "@radix-ui/react-icons";

export const PROVISION_PROD_PAGE_NAME = "production";

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
        deploymentType: "prod" | "dev" | "preview";
        kind: "local" | "cloud";
        previewIdentifier?: string | null;
      }
    | undefined;
  useProjectEnvironmentVariables(
    projectId?: number,
    refreshInterval?: number,
  ): { configs: ProjectEnvVarConfig[] } | undefined;
  useHasProjectAdminPermissions(projectId: number | undefined): boolean;
  useIsDeploymentPaused(): boolean | undefined;
  useLogDeploymentEvent(): (msg: string, props?: object | null) => void;
  CloudImport(props: { sourceCloudBackupId: number }): JSX.Element;
  TeamMemberLink(props: {
    memberId?: number | null;
    name: string;
  }): JSX.Element;
  ErrorBoundary(props: {
    children: ReactNode;
    fallback?: FallbackRender;
  }): JSX.Element;
  teamsURI: string;
  projectsURI: string;
  deploymentsURI: string;
  isSelfHosted: boolean;
};

export const DeploymentInfoContext = createContext<DeploymentInfo>(
  undefined as unknown as DeploymentInfo,
);

type ConnectedDeployment = {
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
      deploymentName === PROVISION_PROD_PAGE_NAME
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

const CONNECTION_STATE_CHECK_INTERVAL_MS = 2500;

function DeploymentWithConnectionState({
  deployment,
  children,
}: {
  deployment: ConnectedDeployment;
  children: ReactNode;
}) {
  const { isSelfHosted, captureMessage, addBreadcrumb } = useContext(
    DeploymentInfoContext,
  );
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
        Date.now() - CONNECTION_STATE_CHECK_INTERVAL_MS * 2
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
          } catch (e) {
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
    [deploymentName, deploymentUrl],
  );

  useEffect(() => {
    // Poll `.connectionState()` every 5 seconds. If we're disconnected twice in a row,
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
    }, CONNECTION_STATE_CHECK_INTERVAL_MS);
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
      {isDisconnected &&
        (deploymentName.startsWith("local-") ? (
          <LocalDeploymentDisconnectOverlay />
        ) : isSelfHosted ? (
          <SelfHostedDisconnectOverlay deploymentUrl={deploymentUrl} />
        ) : null)}
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

function LocalDeploymentDisconnectOverlay() {
  const isSafari = useIsSafari();
  const isBrave = useIsBrave();

  return (
    <div
      className="absolute z-50 mt-[3.5rem] flex h-[calc(100vh-3.5rem)] w-full items-center justify-center"
      style={{
        backdropFilter: "blur(0.5rem)",
      }}
    >
      <div className="max-w-prose">
        <h3 className="mb-4">Can’t connect to your local deployment</h3>

        {isSafari ? (
          <>
            <p className="mb-2">
              Safari blocks connections to localhost. We recommend using another
              browser when using local deployments.
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
              Brave blocks connections to localhost by default. We recommend
              using another browser or{" "}
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
              local deployment may be running on a different device, and can
              only be accessed on that machine.
            </p>
          </>
        )}
      </div>
    </div>
  );
}

function SelfHostedDisconnectOverlay({
  deploymentUrl,
}: {
  deploymentUrl: string;
}) {
  return (
    <div
      className="absolute z-50 mt-[3.5rem] flex h-[calc(100vh-3.5rem)] w-full items-center justify-center"
      style={{
        backdropFilter: "blur(0.5rem)",
      }}
    >
      <div className="max-w-prose">
        <h3 className="mb-4">This deployment is not online.</h3>
        <p className="mb-2">
          Check that your Convex server is running and accessible at{" "}
          <code className="text-sm">{deploymentUrl}</code>.
        </p>
        <p>
          If you continue to have issues, try restarting your Convex server.
        </p>
      </div>
    </div>
  );
}
