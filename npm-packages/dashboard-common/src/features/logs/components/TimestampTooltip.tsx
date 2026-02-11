import { Tooltip } from "@ui/Tooltip";
import { formatDistanceToNow } from "date-fns";
import React from "react";

export type TimestampTooltipProps = {
  timestamp: number;
  children: React.ReactNode;
};

/**
 * A sophisticated timestamp tooltip inspired by Vercel's design.
 * Shows Local time, UTC, and Relative time in a clean, tabular format.
 */
export function TimestampTooltip({
  timestamp,
  children,
}: TimestampTooltipProps) {
  const date = new Date(timestamp);

  // Use Intl for robust formatting without extra bulky libs
  const makeFormatter = (timeZone?: string) => {
    try {
      return new Intl.DateTimeFormat(undefined, {
        dateStyle: "medium",
        timeStyle: "medium",
        fractionalSecondDigits: 3,
        ...(timeZone ? { timeZone } : {}),
      });
    } catch {
      // Fallback for browsers that don't support fractionalSecondDigits with dateStyle/timeStyle.
      return new Intl.DateTimeFormat(undefined, {
        year: "numeric",
        month: "short",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        ...(timeZone ? { timeZone } : {}),
      });
    }
  };

  const localFormatter = makeFormatter();
  const utcFormatter = makeFormatter("UTC");

  const relativeTime = formatDistanceToNow(date, { addSuffix: true });
  const { timeZone } = Intl.DateTimeFormat().resolvedOptions();

  // Calculate UTC offset (e.g., UTC+5:30)
  const offsetMinutes = -date.getTimezoneOffset();
  const offsetHours = Math.floor(Math.abs(offsetMinutes) / 60);
  const offsetMins = Math.abs(offsetMinutes) % 60;
  const offsetSign = offsetMinutes >= 0 ? "+" : "-";
  const offsetStr = `UTC${offsetSign}${offsetHours}:${offsetMins.toString().padStart(2, "0")}`;

  const tip = (
    <div className="flex flex-col gap-2 p-1 text-left font-sans w-max max-w-none">
      <div className="flex flex-col gap-1 items-start">
        <span className="text-content-secondary uppercase text-[10px] font-bold tracking-wider">
          Local ({timeZone})
        </span>
        <span className="text-content-primary tabular-nums whitespace-nowrap font-mono text-xs">
          {localFormatter.format(date)}
        </span>
      </div>

      <div className="flex flex-col gap-1 items-start">
        <span className="text-content-secondary uppercase text-[10px] font-bold tracking-wider">
          UTC ({offsetStr})
        </span>
        <span className="text-content-primary tabular-nums whitespace-nowrap font-mono text-xs">
          {utcFormatter.format(date)}
        </span>
      </div>

      <div className="border-t border-border-secondary pt-2 mt-1 flex flex-col gap-1 items-start">
        <span className="text-content-secondary uppercase text-[10px] font-bold tracking-wider">
          Relative
        </span>
        <span className="text-content-primary tabular-nums whitespace-nowrap font-medium text-xs">
          {relativeTime}
        </span>
      </div>
    </div>
  );

  return (
    <Tooltip
      tip={tip}
      side="top"
      align="start"
      delayDuration={300} // Slight delay to avoid flickering while scanning
      contentClassName="bg-background-secondary border-border-selected shadow-lg px-3 py-2 w-max max-w-none"
    >
      {children}
    </Tooltip>
  );
}
