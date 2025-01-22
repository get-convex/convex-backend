import { useAuth0 } from "@auth0/auth0-react";

export default function Badge() {
  const { user } = useAuth0();

  return (
    <p className="badge">
      <span>Logged in{user!.name ? ` as ${user!.name}` : ""}</span>
    </p>
  );
}
