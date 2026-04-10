import { useQuery } from "convex/react";
import { Id } from "../convex/_generated/dataModel";
import { api } from "../convex/_generated/api";

export function App() {
  const id = localStorage.getItem("myIDStorage");
  const task = useQuery({
    query: api.tasks.getTask,
    args: { taskId: id as Id<"tasks"> },
    throwOnError: true,
  }).data;
  // ...
}
