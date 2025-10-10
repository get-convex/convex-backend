import React from "react";

export function ChartTooltip({
  active,
  payload,
  label,
  showLegend = false,
  extraContent,
}: {
  active?: boolean;
  label: string;
  payload?: any[];
  extraContent?: React.ReactNode;
  showLegend?: boolean;
}) {
  if (active && payload && payload.length) {
    return (
      <div className="rounded-sm border bg-background-secondary/70 p-2 text-right backdrop-blur-[2px] focus:outline-hidden">
        <div className="text-xs font-semibold text-content-primary">
          {label}
        </div>
        {payload.map((dataPoint) => (
          <div
            key={dataPoint.name}
            className="flex items-center justify-end gap-1 tabular-nums"
          >
            {showLegend && (
              <svg className="w-3" viewBox="0 0 50 50" aria-hidden>
                <circle
                  cx="20"
                  cy="20"
                  r="20"
                  className={dataPoint.className}
                  style={{
                    fill: dataPoint.stroke || dataPoint.fill || undefined,
                  }}
                />
              </svg>
            )}

            {dataPoint.formattedValue
              ? dataPoint.formattedValue
              : `${new Intl.NumberFormat("en-us").format(dataPoint.value)}${
                  dataPoint.name
                }`}
          </div>
        ))}
        {extraContent}
      </div>
    );
  }

  return null;
}
