import { RefObject, useCallback, useEffect, useRef } from "react";

/**
 * Ensures that the scroll position changes automatically when `data` updates
 * so that the topmost row stays at its current position.
 *
 * @param data The source data of the list
 * @param scrollRef The scroll view
 * @param getRowId Returns a unique identifier given a row
 * @param rowHeight The (fixed) height of a row
 * @param onRowChangeAbove Called when a row is added/removed above the scrolling position
 */
export function useMaintainScrollPositionOnChange<T>(
  data: T[],
  scrollRef: RefObject<HTMLElement>,
  getRowId: (row: T) => string,
  rowHeight: number,
  onRowChangeAbove: () => void,
) {
  const computeTopmostRowId = useCallback(() => {
    if (!scrollRef.current) return null;
    const topmostRowIndex = Math.floor(scrollRef.current.scrollTop / rowHeight);
    const topmostRow = data[topmostRowIndex];
    return topmostRow ? getRowId(topmostRow) : null;
  }, [data, getRowId, rowHeight, scrollRef]);

  const topmostRowId = useRef<string | null>(null);
  const ignoreScrollEvent = useRef(false);

  // Remember the topmost row
  useEffect(() => {
    const onScroll = () => {
      if (ignoreScrollEvent.current) {
        // Ignore the scroll events fired when setting scrollTop (https://stackoverflow.com/a/1386750)
        ignoreScrollEvent.current = false;
        return;
      }

      topmostRowId.current = computeTopmostRowId();
    };

    const list = scrollRef.current;
    if (!list) return undefined;
    list.addEventListener("scroll", onScroll);
    return () => list?.removeEventListener("scroll", onScroll);
  }, [computeTopmostRowId, scrollRef]);

  // Enforce stickiness
  useEffect(() => {
    if (!scrollRef.current) {
      return;
    }

    if (!topmostRowId.current || scrollRef.current.scrollTop <= 0) {
      topmostRowId.current = computeTopmostRowId();
    }

    // Exit early if there is nothing to do to avoid searching for the new row position
    const currentTopmostRowId = computeTopmostRowId();
    if (currentTopmostRowId === topmostRowId.current) return;

    // Find the new position of the topmost row and scroll to it
    const newTopmostRowIndex = data.findIndex(
      (row) => getRowId(row) === topmostRowId.current,
    );
    if (newTopmostRowIndex === -1) {
      // Row deleted?
      topmostRowId.current = computeTopmostRowId();
      return;
    }

    ignoreScrollEvent.current = true;
    // eslint-disable-next-line no-param-reassign
    scrollRef.current.scrollTop =
      newTopmostRowIndex * rowHeight +
      (scrollRef.current.scrollTop % rowHeight);
    onRowChangeAbove();
  }, [
    computeTopmostRowId,
    data,
    getRowId,
    rowHeight,
    scrollRef,
    onRowChangeAbove,
  ]);
}
