import { useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export function MyApp() {
  const data = useQuery({
    query: api.myFunctions.sum,
    args: { a: 1, b: 2 },
    throwOnError: true,
  }).data;
  // do something with `data`
}
