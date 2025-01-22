/* eslint-disable react-hooks/rules-of-hooks */

// @snippet start example
import { useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export function App() {
  // the URL `param` might be null
  const param = new URLSearchParams(window.location.search).get("param");
  // ERROR! React Hook "useQuery" is called conditionally. React Hooks must
  // be called in the exact same order in every component render.
  const data = param !== null ? useQuery(api.functions.read, { param }) : null;
  //...
}
// @snippet end example
