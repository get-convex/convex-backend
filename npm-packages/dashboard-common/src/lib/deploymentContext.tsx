import { ConvexProvider, ConvexReactClient } from "convex/react";
import { ConvexHttpClient } from "convex/browser";
import {
  createContext,
  ReactNode,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useState,
} from "react";
import { useRouter } from "next/router";
import { captureMessage } from "@sentry/nextjs";
import { cn } from "lib/cn";
import { LoadingLogo } from "../elements/Loading";

export const PROVISION_PROD_PAGE_NAME = "production";

export type DeploymentInfo = (
  | {
      ok: true;
      deploymentUrl: string;
      adminKey: string;
    }
  | { ok: false; errorCode: string; errorMessage: string }
) & {
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
        auditLogsEnabled?: boolean;
      }
    | undefined;
  useCurrentUsageBanner(teamId: number | null): string | null;
  useCurrentDeployment():
    | {
        id: number;
        name: string;
        projectId: number;
        deploymentType: "prod" | "dev" | "preview";
        kind: "local" | "cloud";
      }
    | undefined;
  useHasProjectAdminPermissions(projectId: number | undefined): boolean;
  useIsDeploymentPaused(): boolean | undefined;
  CloudImport(props: { sourceCloudBackupId: number }): JSX.Element;
  TeamMemberLink(props: {
    memberId?: number | null;
    name: string;
  }): JSX.Element;
  projectsURI: string;
  deploymentsURI: string;
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
  errorKind: "None" | "DoesNotExist";
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
    captureMessage(
      `Can't connect to deployment ${connected?.errorCode} ${connected?.errorMessage}`,
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
  const { client, deploymentUrl, deploymentName } = deployment;
  const [connectionState, setConnectionState] = useState<
    "Connected" | "Disconnected" | "LocalDeploymentMismatch" | null
  >(null);
  const [isDisconnected, setIsDisconnected] = useState(false);
  useEffect(() => {
    const checkConnection = setInterval(async () => {
      if (connectionState === "LocalDeploymentMismatch") {
        // Connection status doesn't matter since we're connected to the wrong deployment
        return;
      }

      // Check WS connection status -- if we're disconnected twice in a row, treat
      // the deployment as disconnected.
      const nextConnectionState = client.connectionState();
      if (
        nextConnectionState.isWebSocketConnected === false &&
        connectionState === "Disconnected"
      ) {
        setIsDisconnected(true);
      }
      if (nextConnectionState.isWebSocketConnected === true) {
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
              setConnectionState("LocalDeploymentMismatch");
              setIsDisconnected(true);
              return;
            }
          }
        }
        setIsDisconnected(false);
      }
      setConnectionState(
        nextConnectionState.isWebSocketConnected ? "Connected" : "Disconnected",
      );
    }, 5000);
    return () => clearInterval(checkConnection);
  });
  const value = useMemo(
    () => ({
      deployment,
      isDisconnected,
    }),
    [deployment, isDisconnected],
  );
  return (
    <>
      {isDisconnected && deploymentName.startsWith("local-") ? (
        <LocalDeploymentDisconnectOverlay />
      ) : null}
      <ConnectedDeploymentContext.Provider value={value}>
        {children}
      </ConnectedDeploymentContext.Provider>
    </>
  );
}

function LocalDeploymentDisconnectOverlay() {
  return (
    <div
      className="absolute z-50 flex h-screen w-screen items-center justify-center"
      style={{
        backdropFilter: "blur(0.5rem)",
      }}
    >
      <div className="mt-[-3.5rem]  max-w-[40rem]">
        <h3>You are disconnected from your local deployment!</h3>
        <p className="mb-2">
          Check that <code className="text-sm">npx convex dev</code> is running
          successfully.
        </p>
        <p>
          If you have multiple devices you use with this Convex project, the
          local deployment may be running on a different device, and can only be
          accessed on that machine.
        </p>
      </div>
    </div>
  );
}
