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
  wrapsButton = false,
}: {
  children: React.ReactNode;
  tip: React.ReactNode | undefined;
  side?: TooltipSide;
  align?: "start" | "end" | "center";
  className?: string;
  wrapsButton?: boolean;
}) {
  // Some existing callsites pass in boolean so we do a truthy check
  if (!tip) {
    return <>{children}</>;
  }
  return (
    <RadixTooltip.Provider delayDuration={0}>
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
            className="z-50 max-w-[16rem] break-words rounded border bg-background-secondary/70 p-1 text-center text-xs shadow-sm backdrop-blur-[2px] transition-opacity"
            sideOffset={5}
          >
            {tip}
          </RadixTooltip.Content>
        </RadixTooltip.Portal>
      </RadixTooltip.Root>
    </RadixTooltip.Provider>
  );
}
