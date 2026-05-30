import React from "react";
import { cn } from "@ui/cn";

export type HighlightedTextProps = {
  text: string;
  highlight?: string;
  className?: string;
  markClassName?: string;
};

/**
 * A sophisticated text renderer that highlights occurrences of a substring.
 * Uses <mark> tags with customizable styling.
 */
export function HighlightedText({
  text,
  highlight,
  className,
  markClassName,
}: HighlightedTextProps) {
  if (!highlight || !text) {
    return <span className={className}>{text}</span>;
  }

  // Escape regex special characters from highlight string to prevent crashes on patterns like "[" or "*"
  const escapedHighlight = highlight.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  
  try {
    const parts = text.split(new RegExp(`(${escapedHighlight})`, "gi"));
    
    return (
      <span className={className}>
        {parts.map((part, i) =>
          part.toLowerCase() === highlight.toLowerCase() ? (
            <mark
              key={i}
              className={cn(
                "bg-yellow-300/40 dark:bg-yellow-500/30 text-inherit rounded-xs px-0.5 -mx-0.5 font-medium transition-colors",
                markClassName,
              )}
            >
              {part}
            </mark>
          ) : (
            part
          ),
        )}
      </span>
    );
  } catch (e) {
    // Fallback in case of regex issues
    return <span className={className}>{text}</span>;
  }
}
