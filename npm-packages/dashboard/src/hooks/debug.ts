import { useRef } from "react";

const firstRender = {};
export function useWhatChanged(obj: Record<string, any>, label?: string) {
  const previous = useRef<Record<string, any>>(firstRender);

  if (previous.current === firstRender) {
    previous.current = obj;
  }

  if (Object.keys(obj).length !== Object.keys(previous.current).length) {
    // eslint-disable-next-line no-console
    console.log("changed size!");
    previous.current = obj;
    return;
  }

  const changes: Record<string, [any, any]> = {};
  // eslint-disable-next-line no-restricted-syntax
  for (const prop of Object.keys(obj)) {
    const prev = previous.current[prop];
    const cur = obj[prop];
    if (cur !== prev) {
      changes[prop] = [prev, cur];
    }
  }

  if (Object.keys(changes).length) {
    // eslint-disable-next-line no-console
    console.log(`${label || "component"}rerendered from change:`, changes);
  }

  previous.current = obj;
}
