import React, {
  useState,
  useEffect,
  useCallback,
  ReactNode,
  useMemo,
} from "react";
import { WorkOSSession } from "server/workos";
import { User } from "@workos-inc/node";
import { AuthContext, AuthContextType } from "./AuthContext";

interface AuthProviderProps {
  children: ReactNode;
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [session, setSession] = useState<WorkOSSession | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  const fetchSession = useCallback(async () => {
    try {
      setError(null);
      const response = await fetch("/api/auth/session");

      if (response.ok) {
        const sessionData = await response.json();
        setSession(sessionData);
      } else if (response.status === 401) {
        setSession(null);
      } else {
        throw new Error("Failed to fetch session");
      }
    } catch (err) {
      setError(err as Error);
      setSession(null);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const refreshAuth = useCallback(async () => {
    setIsLoading(true);
    await fetchSession();
  }, [fetchSession]);

  useEffect(() => {
    void fetchSession();
  }, [fetchSession]);

  const user: User | null = useMemo(
    () => (session ? (session.user as User) : null),
    [session],
  );

  const value: AuthContextType = useMemo(
    () => ({
      user,
      isAuthenticated: !!session,
      isLoading,
      error,
      refreshAuth,
    }),
    [user, session, isLoading, error, refreshAuth],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
