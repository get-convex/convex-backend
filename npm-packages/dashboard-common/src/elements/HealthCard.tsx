import {
  CrossCircledIcon,
  ExclamationTriangleIcon,
  InfoCircledIcon,
} from "@radix-ui/react-icons";
import { cn } from "@ui/cn";
import React from "react";
import { Sheet } from "@ui/Sheet";
import { Tooltip } from "@ui/Tooltip";
import { LoadingTransition } from "@ui/Loading";

export function HealthCard({
  title,
  tip,
  children,
  className,
  error,
  warning,
  size = "md",
  action,
}: {
  title: string;
  children?: React.ReactNode;
  className?: string;
  error?: React.ReactNode;
  warning?: React.ReactNode;
  size?: "xs" | "sm" | "md" | "lg";
  action?: React.ReactNode;
  tip?: React.ReactNode;
}) {
  return (
    <Sheet
      padding={false}
      className={cn(
        "flex w-full min-w-48 animate-fadeInFromLoading flex-col transition-all",
        size === "xs" && "h-fit",
        size === "sm" && "min-h-fit",
        size === "md" && "max-h-72 min-h-36",
        size === "lg" && "w-full max-h-[21rem] min-h-fit",
      )}
    >
      <div className="relative flex grow flex-col transition-all">
        <div className="flex items-center justify-between gap-2 p-2">
          <h5 className="truncate">{title}</h5>
          <div className="flex items-center gap-1">
            {warning && (
              <Tooltip
                className="flex gap-1 rounded border bg-background-warning p-0.5 text-xs text-content-warning"
                tip={<div>{warning}</div>}
              >
                <ExclamationTriangleIcon />
              </Tooltip>
            )}
            {error && (
              <Tooltip
                className="flex gap-1 rounded border bg-background-error p-0.5 text-xs text-content-error"
                tip={<div>{error}</div>}
              >
                <CrossCircledIcon />
              </Tooltip>
            )}
            {action}
            {tip && (
              <Tooltip tip={tip} className="border border-transparent p-1">
                <InfoCircledIcon />
              </Tooltip>
            )}
          </div>
        </div>
        {size !== "xs" && (
          <LoadingTransition>
            {children && (
              <div
                className={cn(
                  "flex grow flex-col items-center justify-center",
                  className,
                )}
              >
                {children}
              </div>
            )}
          </LoadingTransition>
        )}
      </div>
    </Sheet>
  );
}
