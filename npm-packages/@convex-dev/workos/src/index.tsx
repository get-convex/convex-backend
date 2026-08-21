import { useCallback, useMemo } from "react";
import { type ReactNode } from "react";
import { LoginRequiredError } from "@workos-inc/authkit-react";
import { ConvexProviderWithAuth, type AuthTokenFetcher } from "convex/react";

type IConvexReactClient = {
  setAuth(fetchToken: AuthTokenFetcher): void;
  clearAuth(): void;
};

// Modified to match WorkOS's auth hook structure
type UseAuth = () => {
  isLoading: boolean;
  user: any | null;
  getAccessToken: (options?: {
    forceRefresh?: boolean;
  }) => Promise<string | null>;
};

/**
 * A wrapper React component which provides a {@link react.ConvexReactClient}
 * authenticated with WorkOS AuthKit.
 *
 * It must be wrapped by a configured `AuthKitProvider`, from
 * `@workos-inc/authkit-react`.
 *
 * @public
 */
export function ConvexProviderWithAuthKit({
  children,
  client,
  useAuth,
}: {
  children: ReactNode;
  client: IConvexReactClient;
  useAuth: UseAuth;
}) {
  const useAuthFromWorkOS = useUseAuthFromAuthKit(useAuth);
  return (
    <ConvexProviderWithAuth client={client} useAuth={useAuthFromWorkOS}>
      {children}
    </ConvexProviderWithAuth>
  );
}

function useUseAuthFromAuthKit(useAuth: UseAuth) {
  return useMemo(
    () =>
      function useAuthFromWorkOS() {
        const { isLoading, user, getAccessToken } = useAuth();

        const fetchAccessToken = useCallback(
          async ({ forceRefreshToken }: { forceRefreshToken: boolean }) => {
            try {
              return await (forceRefreshToken
                ? getAccessToken({ forceRefresh: true })
                : getAccessToken());
            } catch (error) {
              if (error instanceof LoginRequiredError) {
                return null;
              }
              throw error;
            }
          },
          [getAccessToken],
        );

        return useMemo(
          () => ({
            isLoading,
            isAuthenticated: !!user,
            fetchAccessToken,
          }),
          [isLoading, user, fetchAccessToken],
        );
      },
    [useAuth],
  );
}
