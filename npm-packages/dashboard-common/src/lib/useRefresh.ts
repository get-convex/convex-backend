import { useEffect, useReducer } from "react";

let oncePerSecondTimer: ReturnType<typeof setInterval>;
let nextId = 1;
const callbacks = new Map<number, () => void>();
function callCallbacks() {
  for (const cb of callbacks.values()) {
    cb();
  }
}
export function useRefresh(timeout = 1000) {
  const [, forceUpdate] = useReducer((x) => x + 1, 0);
  useEffect(() => {
    const id = nextId++;
    if (callbacks.size === 0) {
      oncePerSecondTimer = setInterval(callCallbacks, timeout);
    }
    callbacks.set(id, forceUpdate);

    return () => {
      callbacks.delete(id);
      if (callbacks.size === 0) {
        clearInterval(oncePerSecondTimer);
      }
    };
  }, [forceUpdate, timeout]);
}
