import React from "react";
import CopyButton from "@theme-original/CodeBlock/CopyButton";

export function CodeWithCopyButton({ text }: { text: string }) {
  return (
    <code className="convex-inline-code-with-copy-button">
      {text}
      <div className="convex-inline-code-copy-button">
        <CopyButton code={text} />
      </div>
    </code>
  );
}
