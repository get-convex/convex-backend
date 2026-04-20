import type { MetaFunction } from "@remix-run/node";
import { api } from "convex/_generated/api";
import { useQuery } from "convex/react";

export const meta: MetaFunction = () => {
  return [
    { title: "New Remix App" },
    { name: "description", content: "Welcome to Remix!" },
  ];
};

export default function Index() {
  const tasks = useQuery(api.tasks.get);
  return (
    <div style={{ fontFamily: "system-ui, sans-serif", lineHeight: "1.8" }}>
      <h1>Welcome to Remix</h1>
      {tasks === undefined
        ? "loading..."
        : tasks.map(({ _id, text }) => <div key={_id}>{text}</div>)}
    </div>
  );
}
