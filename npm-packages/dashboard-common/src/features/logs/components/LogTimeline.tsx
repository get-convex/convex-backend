import React, { useMemo } from "react";
import { InterleavedLog, getTimestamp } from "../lib/interleaveLogs";
import { cn } from "@ui/cn";
import { format } from "date-fns";

export type LogTimelineProps = {
  logs: InterleavedLog[];
  onSelectTimestamp: (ts: number) => void;
  className?: string;
};

const NUM_BUCKETS = 60; // Slightly fewer buckets for better visual separation

export function LogTimeline({
  logs,
  onSelectTimestamp,
  className,
}: LogTimelineProps) {
  const { buckets, minTs, maxTs } = useMemo(() => {
    if (logs.length === 0) return { buckets: [], minTs: 0, maxTs: 0 };

    const timestamps = logs.map(getTimestamp).filter((ts) => ts > 0);
    if (timestamps.length === 0) return { buckets: [], minTs: 0, maxTs: 0 };

    const min = Math.min(...timestamps);
    const max = Math.max(...timestamps);
    const range = max - min || 1;
    const bucketSize = range / NUM_BUCKETS;

    const result = Array.from({ length: NUM_BUCKETS }, (_, i) => ({
      start: min + i * bucketSize,
      end: min + (i + 1) * bucketSize,
      count: 0,
      errorCount: 0,
      logs: [] as InterleavedLog[],
    }));

    for (const log of logs) {
      const ts = getTimestamp(log);
      if (ts === 0) continue;

      let bucketIndex = Math.floor((ts - min) / bucketSize);
      if (bucketIndex >= NUM_BUCKETS) bucketIndex = NUM_BUCKETS - 1;
      if (bucketIndex < 0) bucketIndex = 0;

      result[bucketIndex].count++;
      result[bucketIndex].logs.push(log);

      const isError =
        log.kind === "ExecutionLog" &&
        (log.executionLog.kind === "outcome"
          ? !!log.executionLog.error
          : log.executionLog.output.level === "ERROR");
      if (isError) {
        result[bucketIndex].errorCount++;
      }
    }

    return { buckets: result, minTs: min, maxTs: max };
  }, [logs]);

  if (buckets.length === 0) return null;

  const maxCount = Math.max(...buckets.map((b) => b.count));

  // Generate some time labels for the axis
  const timeLabels = useMemo(() => {
    if (minTs === maxTs) return [];
    const labels = [];
    const count = 5;
    for (let i = 0; i < count; i++) {
      const ts = minTs + (i / (count - 1)) * (maxTs - minTs);
      labels.push({
        label: format(new Date(ts), "HH:mm:ss"),
        position: (i / (count - 1)) * 100,
      });
    }
    return labels;
  }, [minTs, maxTs]);

  return (
    <div className={cn("flex flex-col gap-1.5 w-full", className)}>
      <div
        className={cn(
          "flex h-16 w-full items-end gap-[2px] px-2 py-2 bg-background-secondary/20 rounded-lg border border-border-secondary/30 relative group/timeline shadow-inner",
        )}
      >
        {buckets.map((bucket, i) => {
          const height = maxCount > 0 ? (bucket.count / maxCount) * 100 : 0;
          const hasError = bucket.errorCount > 0;
          const errorRatio = bucket.count > 0 ? bucket.errorCount / bucket.count : 0;

          return (
            <button
              key={i}
              type="button"
              className="group relative flex-1 h-full flex flex-col justify-end overflow-visible focus:outline-none"
              onClick={() => {
                if (bucket.logs.length > 0) {
                  const sorted = [...bucket.logs].sort(
                    (a, b) => getTimestamp(b) - getTimestamp(a),
                  );
                  onSelectTimestamp(getTimestamp(sorted[0]));
                }
              }}
            >
              <div
                className={cn(
                  "w-full transition-all rounded-t-[1.5px] min-h-[1px]",
                  bucket.count > 0 
                    ? (hasError ? "bg-red-500/30" : "bg-content-tertiary/20") 
                    : "bg-transparent",
                  "group-hover:bg-util-accent/40"
                )}
                style={{ height: `${Math.max(height, bucket.count > 0 ? 4 : 0)}%` }}
              >
                {hasError && (
                  <div
                    className="w-full bg-red-500 rounded-t-[1.5px] shadow-[0_0_8px_rgba(239,68,68,0.4)]"
                    style={{ height: `${Math.max(errorRatio * 100, 10)}%` }}
                  />
                )}
              </div>
              
              {/* Tooltip on hover */}
              <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 hidden group-hover:block z-50 pointer-events-none">
                <div className="bg-background-primary border border-border-selected rounded px-2 py-1 shadow-xl text-[10px] whitespace-nowrap flex flex-col gap-0.5">
                  <span className="text-content-secondary">{format(new Date(bucket.start), "HH:mm:ss.SSS")}</span>
                  <div className="flex items-center gap-2">
                    <span className="font-bold">{bucket.count} logs</span>
                    {bucket.errorCount > 0 && (
                      <span className="text-red-500 font-bold">{bucket.errorCount} errors</span>
                    )}
                  </div>
                </div>
                <div className="w-1.5 h-1.5 bg-background-primary border-r border-b border-border-selected rotate-45 absolute -bottom-[4px] left-1/2 -translate-x-1/2" />
              </div>
            </button>
          );
        })}
      </div>
      
      {/* Time Axis */}
      <div className="relative h-4 w-full px-2 text-[10px] text-content-secondary font-mono">
        {timeLabels.map((label, i) => (
          <span
            key={i}
            className="absolute -translate-x-1/2"
            style={{ left: `${label.position}%` }}
          >
            {label.label}
          </span>
        ))}
      </div>
    </div>
  );
}
