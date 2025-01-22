import { useQuery } from "convex/react";
import { Id } from "../convex/_generated/dataModel";
import { api } from "../convex/_generated/api";

export function App() {
  const id = localStorage.getItem("myIDStorage");
  const task = useQuery(api.tasks.getTask, { taskId: id as Id<"tasks"> });
  // ...
}
