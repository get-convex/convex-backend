import React, {
  createContext,
  ReactNode,
  useContext,
  useEffect,
  useState,
} from "react";
import { AuthTokenFetcher } from "../browser/sync/client.js";
import { ConvexProvider } from "./client.js";

// Until we can import from our own entry points (requires TypeScript 4.7),
// just describe the interface enough to help users pass the right type.
type IConvexReactClient = {
  setAuth(
    fetchToken: AuthTokenFetcher,
    onChange: (isAuthenticated: boolean) => void,
  ): void;
  clearAuth(): void;
};

/**
 * Type representing the state of an auth integration with Convex.
 *
 * @public
 */
export type ConvexAuthState = {
  isLoading: boolean;
  isAuthenticated: boolean;
};

const ConvexAuthContext = createContext<ConvexAuthState>(undefined as any);

/**
 * Get the {@link ConvexAuthState} within a React component.
 *
 * This relies on a Convex auth integration provider being above in the React
 * component tree.
 *
 * @returns The current {@link ConvexAuthState}.
 *
 * @public
 */
export function useConvexAuth(): {
  isLoading: boolean;
  isAuthenticated: boolean;
} {
  const authContext = useContext(ConvexAuthContext);
  if (authContext === undefined) {
    throw new Error(
      "Could not find `ConvexProviderWithAuth` (or `ConvexProviderWithClerk` " +
        "or `ConvexProviderWithAuth0`) " +
        "as an ancestor component. This component may be missing, or you " +
        "might have two instances of the `convex/react` module loaded in your " +
        "project.",
    );
  }
  return authContext;
}

/**
 * A replacement for {@link ConvexProvider} which additionally provides
 * {@link ConvexAuthState} to descendants of this component.
 *
 * Use this to integrate any auth provider with Convex. The `useAuth` prop
 * should be a React hook that returns the provider's authentication state
 * and a function to fetch a JWT access token.
 *
 * See [Custom Auth Integration](https://docs.convex.dev/auth/advanced/custom-auth) for more information.
 *
 * @public
 */
export function ConvexProviderWithAuth({
  children,
  client,
  useAuth,
}: {
  children?: ReactNode;
  client: IConvexReactClient;
  useAuth: () => {
    isLoading: boolean;
    isAuthenticated: boolean;
    fetchAccessToken: (args: {
      forceRefreshToken: boolean;
    }) => Promise<string | null>;
  };
}) {
  const { isLoading, isAuthenticated, fetchAccessToken } = useAuth();
  const [isConvexAuthenticated, setIsConvexAuthenticated] = useState<
    boolean | null
  >(null);

  useEffect(() => {
    let isThisEffectRelevant = true;
    if (isAuthenticated) {
      client.setAuth(fetchAccessToken, (isAuthenticated) => {
        if (isThisEffectRelevant) {
          setIsConvexAuthenticated(isAuthenticated);
        }
      });
      return () => {
        isThisEffectRelevant = false;

        // If we haven't finished fetching the token by now
        // we shouldn't transition to a loaded state
        setIsConvexAuthenticated((isConvexAuthenticated) =>
          isConvexAuthenticated ? false : null,
        );
        client.clearAuth();
      };
    }
  }, [isAuthenticated, fetchAccessToken, isLoading, client]);

  // If the useAuth went back to the loading state (which is unusual but possible)
  // reset the Convex auth state to null so that we can correctly
  // transition the state from "loading" to "authenticated"
  // without going through "unauthenticated".
  if (isLoading && isConvexAuthenticated !== null) {
    setIsConvexAuthenticated(null);
  }

  if (!isLoading && !isAuthenticated && isConvexAuthenticated !== false) {
    setIsConvexAuthenticated(false);
  }

  return (
    <ConvexAuthContext.Provider
      value={{
        isLoading: isConvexAuthenticated === null,
        isAuthenticated: isAuthenticated && (isConvexAuthenticated ?? false),
      }}
    >
      <ConvexProvider client={client as any}>{children}</ConvexProvider>
    </ConvexAuthContext.Provider>
  );
}
