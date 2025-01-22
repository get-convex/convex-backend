import { useUser } from "@auth0/nextjs-auth0/client";

export type { UserProfile as User } from "@auth0/nextjs-auth0/client";

// Decorating useUser to make migrating from @auth0-react to @nextjs-auth0 easier
export const useAuth0 = () => {
  const { user, error, isLoading } = useUser();

  return {
    user,
    isAuthenticated: !!user,
    isLoading,
    error,
  };
};
