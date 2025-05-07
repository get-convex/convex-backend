import React from "react";

export function Donut({
  current,
  max,
}: {
  current: number;
  max: number | null | undefined;
}) {
  if (max === null || max === undefined || max === 0) {
    return null;
  }
  // To draw a visible progress arc progress must be <1 and >=0.01
  const progress = Math.max(0.01, Math.min(current / max, 0.99999));
  const isOverHalf = progress >= 0.5;
  const radius = 13;
  const endAngle = 2 * Math.PI * progress - Math.PI / 2;
  const endX = radius * Math.cos(endAngle);
  const endY = radius * Math.sin(endAngle);
  const color = "stroke-util-accent";
  return (
    <div className="relative hidden sm:inline-block">
      <svg
        className="min-h-6 min-w-6"
        width="24"
        height="24"
        viewBox="-16 -16 32 32"
      >
        <circle r="16" className="fill-neutral-2 dark:fill-neutral-4" />
        <circle
          r="10"
          className="fill-background-secondary group-hover:fill-background-primary"
        />
        <path
          d={`M 0 -${radius}
            A ${radius} ${radius} 0 ${isOverHalf ? 1 : 0} 1 ${endX} ${endY}`}
          fill="transparent"
          className={color}
          strokeWidth="6"
        />
      </svg>
    </div>
  );
}
