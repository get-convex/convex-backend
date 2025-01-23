import { useEffect } from "react";

/**
 * Calls a callback when the user clicks/taps outside of a given element or when
 * keyboard focus moves outside.
 */
export function useInteractOutside(
  elementRef: React.RefObject<HTMLElement>,
  callback: () => void,
) {
  useEffect(() => {
    function handleOutsideInteraction(
      event: MouseEvent | TouchEvent | FocusEvent,
    ) {
      if (
        elementRef.current &&
        !elementRef.current.contains(event.target as Node)
      ) {
        callback();
      }
    }

    function handleKeyboardFocus(event: FocusEvent) {
      if (
        elementRef.current &&
        !elementRef.current.contains(event.target as Node)
      ) {
        callback();
      }
    }

    document.addEventListener("mousedown", handleOutsideInteraction);
    document.addEventListener("touchstart", handleOutsideInteraction);
    document.addEventListener("focusin", handleKeyboardFocus);

    return () => {
      document.removeEventListener("mousedown", handleOutsideInteraction);
      document.removeEventListener("touchstart", handleOutsideInteraction);
      document.removeEventListener("focusin", handleKeyboardFocus);
    };
  }, [elementRef, callback]);
}
