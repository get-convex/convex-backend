import { useCallback, useState } from "react";

/**
 * Client-side navigation state for a cursor-paginated list.
 *
 * Keeps a stack of the cursors used to fetch each visited page so the user can
 * step backwards. `currentCursor` feeds the data-fetching hook, so this hook
 * must be called before fetching; the page results (`nextCursor`, emptiness)
 * are wired back in through `onNextPage` and {@link useSnapBackOnEmptyPage}.
 */
export function useCursorPagination() {
  const [cursorHistory, setCursorHistory] = useState<(string | undefined)[]>([
    undefined,
  ]);

  const currentCursor = cursorHistory[cursorHistory.length - 1];
  const currentPage = cursorHistory.length;
  const canGoPrevious = cursorHistory.length > 1;

  const onNextPage = useCallback((nextCursor: string | null | undefined) => {
    if (nextCursor) {
      setCursorHistory((prev) => [...prev, nextCursor]);
    }
  }, []);

  const onPreviousPage = useCallback(() => {
    setCursorHistory((prev) => (prev.length > 1 ? prev.slice(0, -1) : prev));
  }, []);

  const resetPagination = useCallback(() => {
    setCursorHistory([undefined]);
  }, []);

  return {
    currentCursor,
    currentPage,
    canGoPrevious,
    onNextPage,
    onPreviousPage,
    resetPagination,
  };
}
