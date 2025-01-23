import { useCallback, useState } from "react";

export type SelectionState = ReturnType<typeof useSelectionState>;

const EMPTY_SET: Set<string> = new Set();

// Three states:
// 1. All selected
// 2. Some selected
// 3. None selected
// `toggleAll` toggles between 2 -> 3, 3 -> 1 and 1 -> 3
export function useSelectionState(loaded: Set<string>, isAllLoaded: boolean) {
  const [selected, setSelected] = useState<Set<string>>(EMPTY_SET);
  const [allPseudoSelected, setAllPseudoSelected] = useState(false);

  // Clears selected rows that are no longer in the current pagination view.
  // An example case of this happening is a row being deleted by another
  // client, or the filters being changed.
  const [previousLoaded, setPreviousLoaded] = useState(loaded);
  if (previousLoaded !== loaded) {
    setPreviousLoaded(loaded);
    const selectedAndLoaded = [...selected].filter((item) => loaded.has(item));
    if (selectedAndLoaded.length !== selected.size) {
      setSelected(new Set(selectedAndLoaded));
    }
  }

  if (isAllLoaded && allPseudoSelected) {
    setAllPseudoSelected(false);
    setSelected(loaded);
  }

  const toggle = useCallback(
    (item: string) => {
      if (allPseudoSelected) {
        setAllPseudoSelected(false);
      }
      setSelected((selected_) => toggleInSet(selected_, item));
    },
    [allPseudoSelected],
  );

  const toggleAll = useCallback(() => {
    if (isAllLoaded) {
      setSelected((selected_) => (selected_.size === 0 ? loaded : EMPTY_SET));
    } else if (selected.size > 0) {
      setSelected(EMPTY_SET);
    } else {
      setAllPseudoSelected((allPseudoSelected_) => !allPseudoSelected_);
    }
  }, [isAllLoaded, loaded, selected]);

  const has = useCallback(
    (item: string) => allPseudoSelected || selected.has(item),
    [allPseudoSelected, selected],
  );

  const all =
    allPseudoSelected ||
    (isAllLoaded && selected.size > 0 && selected.size === loaded.size
      ? true
      : selected.size > 0
        ? "indeterminate"
        : false);

  const reset = useCallback(() => {
    setSelected(EMPTY_SET);
    setAllPseudoSelected(false);
  }, []);
  return [
    selected,
    {
      has,
      toggle,
      allPseudoSelected,
      all,
      toggleAll,
      reset,
      isExhaustive: isAllLoaded,
    },
  ] as const;
}

function toggleInSet<T>(set: Set<T>, item: T) {
  const clone = new Set(set);
  if (clone.has(item)) {
    clone.delete(item);
  } else {
    clone.add(item);
  }
  return clone;
}
