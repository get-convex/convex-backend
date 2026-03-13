import { cn } from "@ui/cn";
import { useCallback, useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";

export type SegmentedControlOption<T extends string> = {
  label: string;
  value: T;
};

export function SegmentedControl<T extends string>({
  options,
  value,
  onChange,
  className,
}: {
  options: SegmentedControlOption<T>[];
  value: T;
  onChange: (value: T) => void;
  className?: string;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const buttonRefs = useRef<Map<T, HTMLButtonElement>>(new Map());
  const [highlightStyle, setHighlightStyle] = useState<{
    left: number;
    width: number;
  } | null>(null);
  const [animate, setAnimate] = useState(true);

  const measureHighlight = useCallback(() => {
    const container = containerRef.current;
    const button = buttonRefs.current.get(value);
    if (container && button) {
      const containerRect = container.getBoundingClientRect();
      const buttonRect = button.getBoundingClientRect();
      return {
        left: buttonRect.left - containerRect.left,
        width: buttonRect.width,
      };
    }
    return null;
  }, [value]);

  // Animate when value or options change
  useEffect(() => {
    setAnimate(true);
    setHighlightStyle(measureHighlight());
  }, [measureHighlight, options]);

  // Instantly reposition on resize (no animation)
  // This is done to avoid having a frame where the
  // highlighted section overlaps text from a different option
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const observer = new ResizeObserver(() => {
      flushSync(() => {
        setAnimate(false);
        setHighlightStyle(measureHighlight());
      });
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, [measureHighlight]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    const currentIndex = options.findIndex((o) => o.value === value);
    let nextIndex: number | null = null;

    if (e.key === "ArrowRight" || e.key === "ArrowDown") {
      nextIndex = (currentIndex + 1) % options.length;
    } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
      nextIndex = (currentIndex - 1 + options.length) % options.length;
    }

    if (nextIndex !== null) {
      e.preventDefault();
      const nextOption = options[nextIndex];
      onChange(nextOption.value);
      buttonRefs.current.get(nextOption.value)?.focus();
    }
  };

  return (
    <div
      ref={containerRef}
      role="radiogroup"
      className={cn(
        "relative inline-flex rounded-full border bg-background-tertiary p-1",
        className,
      )}
    >
      {highlightStyle && (
        <div
          className={cn(
            "absolute top-1 bottom-1 rounded-full bg-background-secondary shadow-sm",
            animate && "transition-all duration-200 ease-in-out",
          )}
          style={{
            left: highlightStyle.left,
            width: highlightStyle.width,
          }}
        />
      )}
      {options.map((option) => (
        <button
          key={option.value}
          ref={(el) => {
            if (el) {
              buttonRefs.current.set(option.value, el);
            } else {
              buttonRefs.current.delete(option.value);
            }
          }}
          type="button"
          role="radio"
          aria-checked={value === option.value}
          tabIndex={value === option.value ? 0 : -1}
          className={cn(
            "relative z-10 rounded-full px-4 py-1.5 text-sm font-medium transition-colors duration-200 outline-none focus-visible:ring-2 focus-visible:ring-border-selected focus-visible:ring-inset",
            "text-content-primary",
          )}
          onClick={() => onChange(option.value)}
          onKeyDown={handleKeyDown}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
