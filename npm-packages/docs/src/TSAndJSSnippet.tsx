import React from "react";
import { useSelectedDialect } from "./theme/Root";
import { Snippet } from "./snippet";
import { convertFilePath } from "./LanguageSelector";
import { ensureSameLineCount } from "./TSAndJSCode";

// If you need to provide different code for TS vs JS, and you're
// importing the code from a file from the monorepo, use this component.
// Otherwise use normal MDX ``` code block or TSAndJSCode.
export function TSAndJSSnippet({
  sourceJS,
  sourceTS,
  title,
  jsExtension,
  ...props
}: Omit<React.ComponentProps<typeof Snippet>, "source"> & {
  sourceJS: string;
  sourceTS: string;
  // If the JS extension isn't normal translation of the TS extension,
  // like tsx -> js
  jsExtension?: string;
}) {
  const selectedDialect = useSelectedDialect();
  const [ts, js] = ensureSameLineCount(sourceTS, sourceJS);
  return (
    <Snippet
      showLanguageSelector
      source={selectedDialect === "TS" ? ts : js}
      title={convertFilePath(title, selectedDialect, jsExtension)}
      {...props}
    />
  );
}
