import { ReactNode } from "@mdx-js/react/lib";
import React from "react";

export function TourGuide({
  children,
  signPost,
  title,
}: {
  signPost: string;
  title: string;
  children: ReactNode;
}) {
  const childArray = React.Children.toArray(children);
  const steps = childArray.slice(0, -1);
  const illustration = childArray.slice(-1)[0];

  return (
    <div className="convex-tour-guide">
      <div>
        <h5>{signPost}</h5>
        <h3>{title}</h3>
        <ol>{steps}</ol>
      </div>
      <div>{illustration}</div>
    </div>
  );
}

export function TourStep({
  children,
  title,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <li>
      {children !== undefined ? <strong>{title}</strong> : title}
      {children}
    </li>
  );
}
