import { useQuery, useMutation } from "convex/react";
import { api } from "../convex/_generated/api";

export default function App() {
  // Watch the results of the Convex function `getCounter`.
  const counter = useQuery(api.getCounter) ?? 0;

  const increment = useMutation(api.incrementCounter);
  function incrementCounter() {
    // Execute the Convex function `incrementCounter` as a mutation
    // that updates the counter value.
    return increment({ increment: 1 });
  }

  return (
    <main>
      <div>Here's the counter:</div>
      <div>{counter}</div>
      <button onClick={incrementCounter}>Add One!</button>
    </main>
  );
}
