import React, { ReactNode } from "react";
import { useSelectedDialect } from "./theme/Root";
import CodeBlock from "./theme/CodeBlock/CodeBlock";

// If you need to provide different code for TS vs JS, and you're not
// using an import file from the monorepo, use this component.
// Otherwise use normal MDX ``` code block or TSAndJSSnippet.
export function TSAndJSCode({ children }) {
  const childArray = React.Children.toArray(children);
  const selectedDialect = useSelectedDialect();
  if (childArray.length !== 2) {
    throw new Error(
      `JSDialectVariants expects 2 children, got ${childArray.length}`,
    );
  }

  const [ts, js] = ensureSameLineCount(...childArray.map(getCode));

  return (
    <CodeBlock
      showLanguageSelector
      className={childArray[0].props.children.props.className}
      metastring={childArray[0].props.children.props.metastring}
    >
      {selectedDialect === "TS" ? ts : js}
    </CodeBlock>
  );
}

// Adds empty lines to make sure all sources have the same number
// of lines.
export function ensureSameLineCount(...sources: string[]) {
  const maxLineCount = Math.max(
    ...sources.map((code) => code.split("\n").length),
  );
  return sources.map((source) => padLines(source, maxLineCount));
}

function getCode(child: ReactNode) {
  return child.props.children.props.children;
}

function padLines(code: string, maxLineCount: number) {
  const lines = code.split("\n");
  return code + "\n".repeat(maxLineCount - lines.length);
}
