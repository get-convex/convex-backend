import { useEffect, useState } from "react";
import { useDebounce } from "react-use";

// Hook to set a boolean to false, but only after a debounced delay.
export function useEnabledDebounced(enabled: boolean) {
  const [debounced, setDebounced] = useState(enabled);
  useEffect(() => {
    if (!debounced && enabled) {
      setDebounced(true);
    }
  }, [enabled, debounced]);
  useDebounce(() => !enabled && setDebounced(enabled), 100, [enabled]);
  return debounced;
}
