import { useEffect, useRef } from "react";

/**
 * Hook that manages scrolling an element into view and focusing a button
 * when the focused state changes.
 *
 * This consolidates the common pattern of:
 * 1. Focusing a button element when focused becomes true
 * 2. Scrolling the container element into view only on transition to focused
 *
 * @param focused - Whether the element should be focused
 * @param enabled - Whether the focus and scroll behavior is enabled (default: true)
 * @returns Refs for the container element and button element
 */
export function useScrollIntoViewAndFocus({
  focused,
  enabled = true,
}: {
  focused: boolean;
  enabled?: boolean;
}) {
  const elementRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const prevFocusedRef = useRef(false);

  useEffect(() => {
    // Only act on transition to focused (not already focused)
    if (focused && enabled && !prevFocusedRef.current) {
      // Focus the button
      buttonRef.current?.focus();

      // Scroll into view
      elementRef.current?.scrollIntoView({
        block: "nearest",
        inline: "nearest",
      });
    }
    prevFocusedRef.current = focused;
  }, [focused, enabled]);

  return { elementRef, buttonRef };
}
