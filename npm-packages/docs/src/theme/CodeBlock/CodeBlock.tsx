import { LanguageSelector, convertFilePath } from "@site/src/LanguageSelector";
import CodeBlock from "@theme-original/CodeBlock";
import React, { ReactNode } from "react";
import { useSelectedDialect } from "../Root";

export default function CodeBlockWrapper({
  metastring,
  showLanguageSelector,
  title: titleProp,
  ...props
}: {
  className?: string;
  metastring?: string;
  originalType?: string;
  showLanguageSelector?: boolean;
  title?: ReactNode;
  children?: ReactNode;
}) {
  const [_, language] = props.className?.match(/language-(\w+)/) ?? [];
  const title = parseCodeBlockTitle(metastring) ?? titleProp;
  const shouldShowLanguageSelector =
    showLanguageSelector === true ||
    (showLanguageSelector !== false &&
      title !== undefined &&
      title !== null &&
      !shouldNotVary(metastring) &&
      (language === "tsx" || language === "ts"));

  const selectedDialect = useSelectedDialect();

  return (
    <CodeBlock
      title={
        (shouldShowLanguageSelector ? (
          <div className="codeblock-header">
            <div>
              {typeof title === "string"
                ? convertFilePath(title, selectedDialect)
                : title}
            </div>
            <LanguageSelector />
          </div>
        ) : (
          title
        )) as unknown as string
      }
      {...props}
    />
  );
}

export function parseCodeBlockTitle(metastring?: string) {
  return metastring?.match(codeBlockTitleRegex)?.groups!.title;
}

export function shouldNotVary(metastring?: string) {
  return metastring?.includes("noDialect");
}

const codeBlockTitleRegex = /title=(?<quote>["'])(?<title>.*?)\1/;
