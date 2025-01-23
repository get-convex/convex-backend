import isEqual from "lodash/isEqual";
import { DependencyList, EffectCallback, useEffect, useRef } from "react";

export function useDeepEqualsEffect<TDeps extends DependencyList>(
  effect: EffectCallback,
  deps: TDeps,
) {
  const ref = useRef<TDeps | undefined>(undefined);

  if (ref.current === undefined || !isEqual(deps, ref.current)) {
    ref.current = deps;
  }

  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(effect, ref.current);
}
