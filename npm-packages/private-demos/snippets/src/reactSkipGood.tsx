import { useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export function App() {
  const param = new URLSearchParams(window.location.search).get("param");
  const data = useQuery(
    param !== null
      ? { query: api.functions.read, args: { param }, throwOnError: true }
      : "skip",
  )?.data;
  //...
}
