import { ROUTABLE_HTTP_METHODS } from "convex/server";
import { useHoverDirty } from "react-use";
import { useRef } from "react";
import { cn } from "@ui/cn";
import {
  displayName,
  functionIdentifierFromValue,
} from "@common/lib/functions/generateFileTree";
import { Tooltip } from "@ui/Tooltip";
import { PuzzlePieceIcon } from "@common/elements/icons";

function splitFunctionName(functionName: string) {
  for (const method of ROUTABLE_HTTP_METHODS) {
    if (functionName.startsWith(`${method} `)) {
      return {
        secondary: `${method} `,
        primary: functionName.substring(method.length + 1),
      };
    }
  }

  const separatorPos = functionName.indexOf(":");
  if (separatorPos === -1) {
    return { secondary: "", primary: functionName };
  }

  return {
    secondary: functionName.substring(0, separatorPos + 1),
    primary: functionName.substring(separatorPos + 1),
  };
}

const MAX_CHARS_SHOWN = 36;

export function FunctionNameOption({
  label,
  oneLine = false,
  disableTruncation = false,
  maxChars = MAX_CHARS_SHOWN,
  error = false,
}: {
  label: string;
  oneLine?: boolean;
  disableTruncation?: boolean;
  maxChars?: number;
  error?: boolean;
}) {
  const functionIdentifier = functionIdentifierFromValue(label);
  const { componentPath } = functionIdentifier;
  let { primary, secondary } = splitFunctionName(
    displayName(functionIdentifier.identifier),
  );

  const hoverPrimary = primary;
  const hoverSecondary = secondary;

  // Leave some room for the component icon if there is a component path.
  const maxCharsShown = componentPath ? maxChars - 2 : maxChars;

  if (primary.length > maxCharsShown) {
    primary = `...${primary.slice(primary.length - maxCharsShown + 3)}`;

    secondary = "";
  } else if (secondary.length + primary.length > maxCharsShown) {
    secondary = `...${secondary.slice(
      secondary.length - (maxCharsShown - primary.length) + 3,
    )}`;
  }

  const ref = useRef<HTMLDivElement>(null);
  const isHovering = useHoverDirty(ref);

  if (primary === "_other" && secondary === "") {
    return <span className="w-full">Other functions</span>;
  }

  const showHover =
    !disableTruncation &&
    isHovering &&
    (primary !== hoverPrimary || secondary !== hoverSecondary);
  return (
    <div className="flex w-full items-center space-x-1" ref={ref}>
      {componentPath && (
        <Tooltip tip={componentPath}>
          <PuzzlePieceIcon />
        </Tooltip>
      )}
      <div className="group/overlay relative" role="tooltip">
        <span
          className={cn(
            "absolute top-[-3px] hidden rounded-sm border bg-background-secondary p-0.5",
            oneLine
              ? "right-[-3px]"
              : "max-w-full break-words whitespace-normal",
            showHover && "block",
            !disableTruncation &&
              "group-focus/overlay:block group-focus/overlay:ring-3",
          )}
        >
          <span className="text-content-secondary">{hoverSecondary}</span>
          <span className="text-content-primary">{hoverPrimary}</span>
        </span>
        <span aria-label={hoverSecondary + hoverPrimary}>
          <span
            className={error ? "text-content-error" : "text-content-secondary"}
          >
            {disableTruncation ? hoverSecondary : secondary}
          </span>
          <span
            className={error ? "text-content-error" : "text-content-primary"}
          >
            {disableTruncation ? hoverPrimary : primary}
          </span>
        </span>
      </div>
    </div>
  );
}
