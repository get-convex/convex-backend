// This file is not used in the demo app.
// Replace the contents of _auth.tsx with the contents of this file
// to use the default loading and logged out views instead of the custom
// components.

// @snippet start simpleAuthedApp
import { ConvexReactClient } from "convex/react";
import { ConvexProviderWithAuth0 } from "convex/react-auth0";
import { Auth0Provider } from "@auth0/auth0-react";
import { AppProps } from "next/app";

const convex = new ConvexReactClient(process.env.NEXT_PUBLIC_CONVEX_URL!);

export default function MyApp({ Component, pageProps }: AppProps) {
  return (
    <Auth0Provider
      domain={process.env.NEXT_PUBLIC_AUTH0_DOMAIN!}
      clientId={process.env.NEXT_PUBLIC_AUTH0_CLIENT_ID!}
      authorizationParams={{
        redirect_uri:
          typeof window === "undefined" ? undefined : window.location.origin,
      }}
      useRefreshTokens={true}
      cacheLocation="localstorage"
    >
      <ConvexProviderWithAuth0 client={convex}>
        <Component {...pageProps} />
      </ConvexProviderWithAuth0>
    </Auth0Provider>
  );
}
// @snippet end simpleAuthedApp
