import { displayObjectFieldSchema, prettier } from "@common/lib/format";
import { ReadonlyCode } from "@common/elements/ReadonlyCode";
import { Tooltip } from "@ui/Tooltip";
import type { ObjectFieldType } from "convex/values";
import React from "react";

export function ValidatorTooltip({
  fieldSchema,
  columnName,
  children,
  disableTooltip = false,
}: {
  fieldSchema: ObjectFieldType;
  columnName: string;
  children?: React.ReactNode;
  disableTooltip?: boolean;
}) {
  const validatorText = fieldSchema
    ? prettier(displayObjectFieldSchema(fieldSchema), 40).slice(0, -1)
    : null;
  const maxLineWidth = validatorText
    ? validatorText
        .split("\n")
        .reduce((max, line) => Math.max(max, line.length), 0)
    : 0;
  const validatorTooltip = validatorText ? (
    <div className="min-w-fit animate-fadeInFromLoading p-2 text-left">
      <p className="mb-1 text-xs font-semibold whitespace-nowrap">
        Schema for {columnName}:
      </p>
      <div style={{ width: maxLineWidth * 8 }}>
        <ReadonlyCode
          disableLineNumbers
          code={validatorText}
          path={`validator-${columnName}`}
          height={{ type: "content", maxHeightRem: 20 }}
        />
      </div>
    </div>
  ) : null;

  return (
    <Tooltip
      tip={disableTooltip ? undefined : validatorTooltip}
      // Override the default max width to none
      maxWidthClassName=""
      delayDuration={500}
    >
      {children}
    </Tooltip>
  );
}
