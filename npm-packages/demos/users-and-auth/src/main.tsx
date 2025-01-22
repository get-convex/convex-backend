import { Auth0Provider } from "@auth0/auth0-react";
import {
  Authenticated,
  AuthLoading,
  ConvexReactClient,
  Unauthenticated,
} from "convex/react";
import { ConvexProviderWithAuth0 } from "convex/react-auth0";
import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import LoginPage from "./LoginPage";

const convex = new ConvexReactClient(import.meta.env.VITE_CONVEX_URL);

ReactDOM.createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Auth0Provider
      // Replace these with your own Auth0 Domain and Client ID
      // or with `{import.meta.env.VITE_AUTH0_DOMAIN}` and
      // `{import.meta.env.VITE_AUTH0_CLIENT_ID}`
      // and configure VITE_AUTH0_DOMAIN and VITE_AUTH0_CLIENT_ID
      // in your .env.local
      domain="dev-dr8esswf5jyzlaf6.us.auth0.com"
      clientId="8DJpkTAjDwR9VOzNfyN7PLzhX3zcB7fd"
      authorizationParams={{
        redirect_uri: window.location.origin,
      }}
      useRefreshTokens={true}
      cacheLocation="localstorage"
    >
      <ConvexProviderWithAuth0 client={convex}>
        <Authenticated>
          <App />
        </Authenticated>
        <Unauthenticated>
          <LoginPage />
        </Unauthenticated>
        <AuthLoading>
          <main>Loading...</main>
        </AuthLoading>
      </ConvexProviderWithAuth0>
    </Auth0Provider>
  </StrictMode>,
);
