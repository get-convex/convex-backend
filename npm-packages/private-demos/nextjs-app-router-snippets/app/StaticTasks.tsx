import { fetchQuery } from "convex/nextjs";
import { api } from "@/convex/_generated/api";

export async function StaticTasks() {
  const tasks = await fetchQuery(api.tasks.list, { list: "default" });
  // render `tasks`...
  return <div>...</div>;
}
