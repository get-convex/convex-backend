import React, { useEffect } from "react";
import { setUser } from "@sentry/nextjs";
import { useProfile } from "api/profile";

export function SentryUserProvider({
  children,
}: {
  children: React.ReactElement;
}) {
  const profile = useProfile();

  useEffect(() => {
    if (profile) {
      setUser({
        id: profile.id ?? undefined,
        email: profile.email ?? undefined,
      });
    }
  }, [profile]);

  return children;
}
