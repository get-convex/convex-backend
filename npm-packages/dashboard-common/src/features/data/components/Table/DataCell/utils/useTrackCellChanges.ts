import { Value } from "convex/values";
import isEqual from "lodash/isEqual";
import { useState, useEffect } from "react";
import { usePrevious, useFirstMountState } from "react-use";

export function useTrackCellChanges({
  value,
  didRowChange,
}: {
  value: Value;
  didRowChange: boolean;
}) {
  const previousValue = usePrevious(value);
  const [didJustChange, setDidJustChange] = useState(false);
  const mounted = useFirstMountState();
  useEffect(() => {
    if (
      // Don't highlight if the cell is rendering for the first time
      !mounted &&
      // Highlight rows when the value changes
      !didRowChange &&
      !isEqual(value, previousValue)
    ) {
      setDidJustChange(true);
      // To reset the animatation, reset the state after one second.
      setTimeout(() => setDidJustChange(false), 1000);
    }
  }, [previousValue, value, mounted, didRowChange]);

  return didJustChange;
}
