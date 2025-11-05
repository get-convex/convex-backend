import { useCallback, useMemo } from "react";
import { type ReactNode } from "react";
import { ConvexProviderWithAuth, type AuthTokenFetcher } from "convex/react";

type IConvexReactClient = {
  setAuth(fetchToken: AuthTokenFetcher): void;
  clearAuth(): void;
};

// Modified to match WorkOS's auth hook structure
type UseAuth = () => {
  isLoading: boolean;
  user: any | null;
  getAccessToken: () => Promise<string | null>;
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

        const fetchAccessToken = useCallback(async () => {
          try {
            const token = await getAccessToken();
            return token;
          } catch {
            return null;
          }
        }, [getAccessToken]);

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
