import { useAction } from "convex/react";
import { api } from "../convex/_generated/api";

export function MyApp() {
  const performMyAction = useAction(api.myFunctions.doSomething);
  const handleClick = () => {
    performMyAction({ a: 1 });
  };
  // pass `handleClick` to a button
  // ...
}
