import React, { ReactNode } from "react";

export function ErrorExample({
  children,
  name,
}: {
  children: ReactNode;
  name: string;
}) {
  return (
    <blockquote>
      <span style={{ color: "var(--color-error)" }}>failure</span>{" "}
      <code>{name}</code>
      <div style={{ fontSize: "0.9rem" }}>{children}</div>
    </blockquote>
  );
}
