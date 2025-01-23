import { ReactNode } from "@mdx-js/react/lib";
import RawDetails from "@theme-original/Details";
import React from "react";

// Wrapper component for MDXv2
export function Details({
  children,
  summary,
  className,
}: {
  children: ReactNode;
  summary: ReactNode;
  className?: string;
}) {
  return (
    <RawDetails className={className} summary={<summary>{summary}</summary>}>
      {children}
    </RawDetails>
  );
}
