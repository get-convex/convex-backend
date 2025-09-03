import { User } from "@workos-inc/node";
import { useAuth } from "providers/AuthProvider";

interface UseWorkOSReturn {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: Error | null;
}

export const useWorkOS = (): UseWorkOSReturn => {
  const authContext = useAuth();

  return {
    user: authContext.user,
    isAuthenticated: authContext.isAuthenticated,
    isLoading: authContext.isLoading,
    error: authContext.error,
  };
};
