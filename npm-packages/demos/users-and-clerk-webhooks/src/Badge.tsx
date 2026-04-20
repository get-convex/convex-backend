import { useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export default function Badge() {
  const user = useQuery(api.users.current);
  const sentCount = useQuery(api.messages.sentCount);

  return (
    <p className="badge">
      <span>Logged in{user?.name ? ` as ${user.name}` : ""}</span>
      {sentCount !== undefined && <span>Sent {sentCount} messages</span>}
    </p>
  );
}
