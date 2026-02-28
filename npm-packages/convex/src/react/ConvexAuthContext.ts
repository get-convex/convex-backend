import { createContext, useContext } from "react";

/**
 * Type representing the state of an auth integration with Convex.
 *
 * @public
 */
export type ConvexAuthState = {
  isLoading: boolean;
  isAuthenticated: boolean;
};

export const ConvexAuthContext = createContext<ConvexAuthState | undefined>(
  undefined,
);

/**
 * @internal
 */
export function useOptionalConvexAuth(): ConvexAuthState | undefined {
  return useContext(ConvexAuthContext);
}
