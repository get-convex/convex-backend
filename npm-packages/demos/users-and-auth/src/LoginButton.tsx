import { useAuth0 } from "@auth0/auth0-react";
import { useConvexAuth } from "convex/react";

export default function LoginButton() {
  const { isLoading } = useConvexAuth();
  const { loginWithRedirect } = useAuth0();
  return (
    <button disabled={isLoading} onClick={() => loginWithRedirect()}>
      Log in
    </button>
  );
}
