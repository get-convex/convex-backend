import { useGlobalLocalStorage } from "lib/useGlobalLocalStorage";
import { useEffect } from "react";
import { TableState } from "react-table";
import { usePrevious } from "react-use";

export const useTrackColumnWidths = (
  state: TableState<object>,
  localStorageKey: string,
) => {
  const { isResizingColumn } = state.columnResizing;
  const [savedWidths, setSavedWidths] = useGlobalLocalStorage<
    | {
        columnWidths: { [key: string]: number };
      }
    | undefined
  >(localStorageKey, { columnWidths: {} });

  const wasResizingColumn = usePrevious(isResizingColumn);
  useEffect(() => {
    if (
      localStorageKey !== "_disabled" &&
      wasResizingColumn &&
      !isResizingColumn
    ) {
      setSavedWidths({
        columnWidths: {
          ...(savedWidths?.columnWidths || {}),
          [wasResizingColumn]:
            state.columnResizing.columnWidths[wasResizingColumn],
        },
      });
    }
  }, [
    isResizingColumn,
    wasResizingColumn,
    savedWidths,
    setSavedWidths,
    state.columnResizing.columnWidths,
    localStorageKey,
  ]);

  return () => setSavedWidths(undefined);
};
