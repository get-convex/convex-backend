import React, { useEffect } from "react";
import { setUser } from "@sentry/nextjs";
import { useAuth0 } from "hooks/useAuth0";
import { useProfile } from "api/profile";

export function SentryUserProvider({
  children,
}: {
  children: React.ReactElement;
}) {
  const { user } = useAuth0();
  const profile = useProfile();

  useEffect(() => {
    if (user) {
      setUser({
        id: user.sub ?? undefined,
        email: (profile?.email || user.email) ?? undefined,
      });
    }
  }, [profile?.email, user]);

  return children;
}
