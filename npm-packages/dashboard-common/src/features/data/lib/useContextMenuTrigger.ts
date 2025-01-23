import { RefObject, useEffect, useRef } from "react";

// Based on https://codesandbox.io/s/trusting-rui-2duieo
export function useContextMenuTrigger(
  triggerRef: RefObject<HTMLElement>,
  onOpenContextMenu: (position: { x: number; y: number }) => void,
  onCloseContextMenu: () => void,
) {
  // When right-clicking for a long time, the context menu will disappear
  // when the right mouse button is released.
  const allowMouseUpCloseRef = useRef(false);

  useEffect(() => {
    if (!triggerRef || !triggerRef.current) return undefined;
    const trigger = triggerRef.current;

    let timeout: number;

    function onContextMenu(e: MouseEvent) {
      e.preventDefault();

      onOpenContextMenu({
        x: e.clientX,
        y: e.clientY,
      });

      clearTimeout(timeout);

      allowMouseUpCloseRef.current = false;
      timeout = window.setTimeout(() => {
        allowMouseUpCloseRef.current = true;
      }, 300);
    }

    function onMouseUp() {
      if (allowMouseUpCloseRef.current) {
        onCloseContextMenu();
      }
    }

    trigger.addEventListener("contextmenu", onContextMenu);
    trigger.addEventListener("mouseup", onMouseUp);
    return () => {
      trigger.removeEventListener("contextmenu", onContextMenu);
      trigger.removeEventListener("mouseup", onMouseUp);
      clearTimeout(timeout);
    };
  }, [triggerRef, onOpenContextMenu, onCloseContextMenu]);
}
