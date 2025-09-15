"use client";

import { ConvexProvider, ConvexReactClient } from "convex/react";
import { ReactNode, useMemo, useEffect } from "react";

const convexUrl = process.env.NEXT_PUBLIC_CONVEX_URL!;

export const CONVEX_JWT_TOKEN_KEY = "convex_jwt_token";

export function ConvexProviderWithAuth({ children }: { children: ReactNode }) {
  const client = useMemo(() => {
    return new ConvexReactClient(convexUrl, {
      unsavedChangesWarning: false,
    });
  }, []);

  useEffect(() => {
    // Set up auth token callback
    client.setAuth(
      async () => {
        if (typeof window !== "undefined") {
          console.log("SET AUTH");
          return localStorage.getItem(CONVEX_JWT_TOKEN_KEY);
        }
        return null;
      },
      (isAuthenticated) => {
        console.log("Convex authentication status:", isAuthenticated);
      },
    );
  }, [client]);

  return <ConvexProvider client={client}>{children}</ConvexProvider>;
}
