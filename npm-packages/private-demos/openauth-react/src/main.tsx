import { StrictMode, useCallback, useMemo } from "react";
import { createRoot } from "react-dom/client";
import { AuthProvider, useAuth } from "./AuthContext";
import App from "./App";
import { ConvexProviderWithAuth, ConvexReactClient } from "convex/react";
const address = import.meta.env.VITE_CONVEX_URL;
const convex = new ConvexReactClient(address);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AuthProvider>
      <ConvexProviderWithAuth useAuth={useAuthFromOpenAuth} client={convex}>
        <App />
      </ConvexProviderWithAuth>
    </AuthProvider>
  </StrictMode>,
);

function useAuthFromOpenAuth() {
  const auth = useAuth();
  const { loaded, loggedIn, getToken } = auth;
  const fetchAccessToken = useCallback(
    async ({
      forceRefreshToken: _forceRefreshToken,
    }: {
      forceRefreshToken: boolean;
    }) => {
      const token = await getToken();
      console.log("got token!", token);
      // This might *always* get a new JWT, so we may want to cache.
      return token || null;
    },
    // If `getToken` isn't correctly memoized
    // remove it from this dependency array
    [getToken],
  );
  return useMemo(
    () => ({
      isLoading: loaded,
      isAuthenticated: loggedIn,
      fetchAccessToken,
    }),
    [loaded, loggedIn, fetchAccessToken],
  );
}
