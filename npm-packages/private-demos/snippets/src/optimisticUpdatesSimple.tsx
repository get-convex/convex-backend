import { api } from "../convex/_generated/api";
import { useMutation } from "convex/react";

export function IncrementCounter() {
  const increment = useMutation(api.counter.increment).withOptimisticUpdate(
    (localStore, args) => {
      const { increment } = args;
      const currentValue = localStore.getQuery(api.counter.get);
      if (currentValue !== undefined) {
        localStore.setQuery(api.counter.get, {}, currentValue + increment);
      }
    },
  );

  const incrementCounter = () => {
    increment({ increment: 1 });
  };

  return <button onClick={incrementCounter}>+1</button>;
}
