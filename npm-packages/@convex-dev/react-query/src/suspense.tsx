import { useSuspenseQuery } from "@tanstack/react-query";
import { Suspense } from "react";
import { convexQuery } from "./index.js";
import { api } from "../convex/_generated/api.js";

export function SuspenseMessageCount() {
  const { data } = useSuspenseQuery(convexQuery(api.messages.count, {}));

  return <div className="message-count">{data} messages</div>;
}

export function SuspenseMessageCountWithFallback() {
  return (
    <Suspense
      fallback={<div className="message-count">Loading messages...</div>}
    >
      <SuspenseMessageCount />
    </Suspense>
  );
}
