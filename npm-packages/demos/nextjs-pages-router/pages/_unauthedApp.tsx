// This file is not used in the demo app.
// Replace the contents of _auth.tsx with the contents of this file
// to use a Convex provider without authentication.

// @snippet start unauthedApp
import { ConvexProvider, ConvexReactClient } from "convex/react";
import { AppProps } from "next/app";

const convex = new ConvexReactClient(process.env.NEXT_PUBLIC_CONVEX_URL!);

function MyApp({ Component, pageProps }: AppProps) {
  return (
    <ConvexProvider client={convex}>
      <Component {...pageProps} />;
    </ConvexProvider>
  );
}

export default MyApp;
// @snippet end unauthedApp
