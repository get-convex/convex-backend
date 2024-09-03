import React from "react";

import { ReactNode, useCallback, useMemo } from "react";
import { AuthTokenFetcher } from "../browser/sync/client.js";
import { ConvexProviderWithAuth } from "../react/ConvexAuthState.js";

// Until we can import from our own entry points (requires TypeScript 4.7),
// just describe the interface enough to help users pass the right type.
type IConvexReactClient = {
  setAuth(fetchToken: AuthTokenFetcher): void;
  clearAuth(): void;
};

// https://clerk.com/docs/reference/clerk-react/useauth
type UseAuth = () => {
  isLoaded: boolean;
  isSignedIn: boolean | undefined;
  getToken: (options: {
    template?: "convex";
    skipCache?: boolean;
  }) => Promise<string | null>;
};

/**
 * A wrapper React component which provides a {@link react.ConvexReactClient}
 * authenticated with Clerk.
 *
 * It must be wrapped by a configured `ClerkProvider`, from
 * `@clerk/clerk-react`, `@clerk/clerk-expo`, `@clerk/nextjs` or
 * another React-based Clerk client library and have the corresponding
 * `useAuth` hook passed in.
 *
 * See [Convex Clerk](https://docs.convex.dev/auth/clerk) on how to set up
 * Convex with Clerk.
 *
 * @public
 */
export function ConvexProviderWithClerk({
  children,
  client,
  useAuth,
}: {
  children: ReactNode;
  client: IConvexReactClient;
  useAuth: UseAuth;
}) {
  const useAuthFromClerk = useUseAuthFromClerk(useAuth);
  return (
    <ConvexProviderWithAuth client={client} useAuth={useAuthFromClerk}>
      {children}
    </ConvexProviderWithAuth>
  );
}

function useUseAuthFromClerk(useAuth: UseAuth) {
  return useMemo(
    () =>
      function useAuthFromClerk() {
        const { isLoaded, isSignedIn, getToken } = useAuth();
        const fetchAccessToken = useCallback(
          async ({ forceRefreshToken }: { forceRefreshToken: boolean }) => {
            try {
              return getToken({
                template: "convex",
                skipCache: forceRefreshToken,
              });
            } catch {
              return null;
            }
          },
          // Clerk is not memoizing its getToken function at all
          // eslint-disable-next-line react-hooks/exhaustive-deps
          [],
        );
        return useMemo(
          () => ({
            isLoading: !isLoaded,
            isAuthenticated: isSignedIn ?? false,
            fetchAccessToken,
          }),
          [isLoaded, isSignedIn, fetchAccessToken],
        );
      },
    [useAuth],
  );
}
