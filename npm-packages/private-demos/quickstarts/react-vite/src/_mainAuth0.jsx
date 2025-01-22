import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import { ConvexProviderWithAuth0 } from "convex/react-auth0";
import { Auth0Provider } from "@auth0/auth0-react";
import { ConvexReactClient } from "convex/react";

const convex = new ConvexReactClient(import.meta.env.VITE_CONVEX_URL);

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <Auth0Provider
      domain="your-domain.us.auth0.com"
      clientId="yourclientid"
      authorizationParams={{
        redirect_uri: window.location.origin,
      }}
      useRefreshTokens={true}
      cacheLocation="localstorage"
    >
      <ConvexProviderWithAuth0 client={convex}>
        <App />
      </ConvexProviderWithAuth0>
    </Auth0Provider>
  </React.StrictMode>,
);
