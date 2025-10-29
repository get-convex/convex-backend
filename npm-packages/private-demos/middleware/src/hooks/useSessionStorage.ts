import { convexToJson, jsonToConvex, Value } from "convex/values";
import { useCallback, useState } from "react";

function useSessionStorage<T extends Value>(
  key: string,
): [T | undefined, (value: T) => void];
function useSessionStorage<T extends Value>(
  key: string,
  defaultValue: T | (() => T),
): [T, (value: T) => void];
function useSessionStorage<T extends Value>(
  key: string,
  defaultValue?: T | (() => T),
) {
  const [value, setValueInternal] = useState(() => {
    if (typeof sessionStorage !== "undefined") {
      const existing = sessionStorage.getItem(key);
      if (existing) {
        try {
          return jsonToConvex(JSON.parse(existing)) as T;
        } catch (e) {
          console.error(e);
        }
      }
    }
    if (typeof defaultValue === "function") {
      return defaultValue();
    }
    return defaultValue;
  });
  const setValue = useCallback(
    (value: T) => {
      sessionStorage.setItem(key, JSON.stringify(convexToJson(value)));
      setValueInternal(value);
    },
    [key],
  );
  return [value, setValue] as const;
}

export default useSessionStorage;
