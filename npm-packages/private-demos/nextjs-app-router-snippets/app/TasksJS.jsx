"use client";

import { usePreloadedQuery } from "convex/react";

export function Tasks(props) {
  const tasks = usePreloadedQuery(props.preloadedTasks);
  // render `tasks`...
  return tasks.map((task) => <div key={task._id}>{task.text}</div>);
}
