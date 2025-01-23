"use client";

import { Button } from "@/components/ui/button";
import { api } from "@/convex/_generated/api";
import { Preloaded, usePreloadedQuery } from "convex/react";
import { useMutation } from "convex/react";

export function Counter({
  preloadedCounter,
  counterName,
}: {
  counterName: string;
  preloadedCounter: Preloaded<typeof api.counter.get>;
}) {
  const counter = usePreloadedQuery(preloadedCounter);
  const increment = useMutation(api.counter.increment);
  return (
    <div className="bg-violet-300 p-4 rounded-md flex flex-col gap-4">
      <p>
        {"The dynamic value of the counter is:"}{" "}
        <span className="font-bold">{counter}</span>
      </p>
      <div>
        <Button onClick={() => increment({ counterName })}>Add One!</Button>
      </div>
    </div>
  );
}
