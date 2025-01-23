import {
  Callout,
  CopyButton,
  ReadonlyCode,
  HighlightLines,
  displaySchemaFromShapes,
} from "dashboard-common";
import Link from "next/link";
import React, { useMemo } from "react";
import { Shape } from "shapes";
import { ExternalLinkIcon } from "@radix-ui/react-icons";
import { ConvexSchemaFilePath } from "./ConvexSchemaFilePath";

export type LineHighlighter = (code: string) => HighlightLines;
export type CodeTransformation = (code: string) => string;

export function GenerateSchema({
  shapes,
  hadError,
  lineHighlighter = undefined,
  codeTransformation = (code) => code,
  showLearnMoreLink = true,
  showUsageInstructions = true,
}: {
  shapes: Map<string, Shape>;
  hadError: boolean;
  // Determines which lines to highlight in the UI. A number highlights a
  // single line. HightLines higlights a range.  The code provided to this
  // function will have already been mutated by codeTransformation if one is
  // provided.
  lineHighlighter?: LineHighlighter;
  // Optionally transforms the code block this function produces from the given
  // shapes.
  codeTransformation?: CodeTransformation;
  showLearnMoreLink?: boolean;
  showUsageInstructions?: boolean;
}) {
  const displayedSchema = useMemo(() => {
    const schema = displaySchemaFromShapes(shapes);
    return schema ? codeTransformation(schema) : "";
  }, [codeTransformation, shapes]);

  const widthString = displayedSchema ? "max-w-full" : "w-[32rem]";
  const learnMoreLink = (
    <Link
      className="inline-flex items-center text-content-link dark:underline"
      href="https://docs.convex.dev/database/schemas"
      target="_blank"
    >
      Schema docs
      <ExternalLinkIcon className="ml-2" />
    </Link>
  );
  return (
    <div className={widthString}>
      {hadError && (
        <Callout className="mb-2" variant="error">
          Encountered an error generating the table schema.
        </Callout>
      )}
      <div className="mb-2">
        {displayedSchema ? (
          <div className="flex flex-col gap-2">
            <div>
              We've generated a schema based on the data available in your table
              {shapes.size > 1 ? "s" : ""}. {showLearnMoreLink && learnMoreLink}
            </div>
            {showUsageInstructions && (
              <>
                <div>
                  Paste this schema into the{" "}
                  <ConvexSchemaFilePath className="text-xs" /> file in your
                  codebase.{" "}
                </div>
                <div>
                  Modify the field types if the generated schema doesn’t fit
                  your data model.
                </div>
              </>
            )}
          </div>
        ) : (
          <div>
            Your project doesn't have any data yet. Once your tables have data,
            Convex will be able to suggest a schema for your project.
            {showLearnMoreLink && learnMoreLink}
          </div>
        )}
      </div>
      {displayedSchema && (
        <div
          className="relative block whitespace-pre-wrap break-words rounded border p-4 text-sm"
          aria-hidden="true"
        >
          <ReadonlyCode
            path="generateSchema"
            highlightLines={
              lineHighlighter ? lineHighlighter(displayedSchema) : undefined
            }
            code={displayedSchema}
            language="javascript"
            disableLineNumbers
            height={{ type: "content", maxHeightRem: 52 }}
          />
          <div className="absolute right-0 top-0 h-10">
            <CopyButton text={displayedSchema} />
          </div>
        </div>
      )}
    </div>
  );
}
