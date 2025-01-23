import { ReactNode } from "@mdx-js/react/lib";
import React from "react";

export function StepByStep({ children }: { children: ReactNode }) {
  return <ol className="convex-step-by-step">{children}</ol>;
}

export function Step({
  children,
  title,
}: {
  children: ReactNode;
  title: ReactNode;
}) {
  const childArray = React.Children.toArray(children);
  const description = childArray.slice(0, -1);
  const code = childArray.slice(-1)[0];

  return (
    <li>
      <div className="convex-step">
        <div>
          <div style={{ fontWeight: "bold" }}>{title}</div>
          {description}
        </div>
        <div>{code}</div>
      </div>
    </li>
  );
}
