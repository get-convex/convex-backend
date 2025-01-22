import { RefObject, useCallback, useEffect, useState } from "react";
import { ListOnScrollProps, FixedSizeList } from "react-window";
import { usePrevious } from "react-use";
import { InterleavedLog } from "../utils/interleaveLogs";

export function useStickyLogs(
  listRef: RefObject<FixedSizeList>,
  logs: InterleavedLog[],
  scrollThreshhold?: number,
): {
  showNewLogs: (() => void) | null;
  onScroll: (props: ListOnScrollProps) => void;
} {
  const [showButton, setShowButton] = useState(false);

  const [isStuck, setIsStuck] = useState(true);

  useTrackNewLogs(logs, listRef, isStuck, setShowButton);

  return {
    onScroll: useCallback(
      ({ scrollOffset }) => {
        const newIsStuck = isStuckAtBottom(scrollOffset, scrollThreshhold);
        setIsStuck(newIsStuck);
        newIsStuck && setShowButton(false);
      },
      [scrollThreshhold],
    ),
    showNewLogs: showButton
      ? () => {
          setShowButton(false);
          setIsStuck(true);
          listRef.current?.scrollToItem(logs.length, "end");
          // TODO: Figure out a better way to make sure we get pinned to the bottom
          // This is a hack to try and keep up with really fast-scrolling logs when you click the
          // button.
          setTimeout(() => {
            listRef.current?.scrollToItem(logs.length, "end");
          }, 0);
        }
      : null,
  };
}

// Determines the threshold for when the user is considered to be at the bottom of the scroll.
function isStuckAtBottom(newScrollTop?: number, scrollThreshhold?: number) {
  if (scrollThreshhold === undefined || newScrollTop === undefined) return true;
  return newScrollTop >= scrollThreshhold;
}

function useTrackNewLogs(
  logs: InterleavedLog[],
  listRef: RefObject<FixedSizeList>,
  isStuck: boolean,
  setShowButton: (show: boolean) => void,
) {
  const previousLogs = usePrevious(logs);
  useEffect(() => {
    if (!listRef.current || previousLogs?.length === logs.length) return;

    if (isStuck) {
      listRef.current.scrollToItem(logs.length, "end");
    } else {
      setShowButton(true);
    }
  }, [isStuck, listRef, setShowButton, logs.length, previousLogs?.length]);
}
