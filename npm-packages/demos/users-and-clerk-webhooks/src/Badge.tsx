import { useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export default function Badge() {
  const user = useQuery({
    query: api.users.current,
    args: {},
    throwOnError: true,
  }).data;
  const sentCount = useQuery({
    query: api.messages.sentCount,
    args: {},
    throwOnError: true,
  }).data;

  return (
    <p className="badge">
      <span>Logged in{user?.name ? ` as ${user.name}` : ""}</span>
      {sentCount !== undefined && <span>Sent {sentCount} messages</span>}
    </p>
  );
}
