import React from "react";
import * as RadixTooltip from "@radix-ui/react-tooltip";
import classNames from "classnames";

export type TooltipSide = "left" | "right" | "bottom" | "top";

export function Tooltip({
  children,
  tip,
  side = "bottom",
  align = "center",
  className,
  contentClassName,
  wrapsButton = false,
  delayDuration = 0,
  maxWidthClassName = "max-w-[16rem]",
}: {
  children: React.ReactNode;
  tip: React.ReactNode | undefined;
  side?: TooltipSide;
  align?: "start" | "end" | "center";
  className?: string;
  contentClassName?: string;
  maxWidthClassName?: string;
  wrapsButton?: boolean;
  delayDuration?: number;
}) {
  // Some existing callsites pass in boolean so we do a truthy check
  if (!tip) {
    return <>{children}</>;
  }
  return (
    <RadixTooltip.Provider delayDuration={delayDuration}>
      <RadixTooltip.Root>
        <RadixTooltip.Trigger
          asChild={wrapsButton}
          className={classNames("focus-visible:outline-0", className)}
        >
          {children}
        </RadixTooltip.Trigger>
        <RadixTooltip.Portal>
          <RadixTooltip.Content
            side={side}
            align={align}
            className={classNames(
              "z-50 break-words rounded border bg-background-secondary/70 p-1 text-center text-xs shadow-sm backdrop-blur-[2px] transition-opacity",
              maxWidthClassName,
              contentClassName,
            )}
            role="tooltip"
            sideOffset={5}
          >
            {tip}
          </RadixTooltip.Content>
        </RadixTooltip.Portal>
      </RadixTooltip.Root>
    </RadixTooltip.Provider>
  );
}
