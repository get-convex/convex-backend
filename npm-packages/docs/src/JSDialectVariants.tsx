import React from "react";
import { useSelectedDialect } from "./theme/Root";

export function JSDialectVariants({ children }) {
  const childArray = React.Children.toArray(children);
  const selectedDialect = useSelectedDialect();
  if (childArray.length !== 2) {
    throw new Error(
      `JSDialectVariants expects 2 children, got ${childArray.length}`,
    );
  }
  return selectedDialect === "TS" ? childArray[0] : childArray[1];
}
