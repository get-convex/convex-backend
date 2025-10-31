import { useMemo } from "react";
import { GenericDocument } from "convex/server";
import { useLocalStorage } from "react-use";
import { isInCommonUTCTimestampRange } from "@common/features/data/lib/helpers";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";

const MIN_COLUMN_WIDTH = 50;
// Accounts for different locales, prevents truncation if _creationTime is the last column
const CREATION_TIME_COLUMN_MIN_WIDTH = 200;
const MAX_COLUMN_WIDTH = 1000;
const DEFAULT_COLUMN_WIDTH = 150;
const ID_COLUMN_WIDTH = 100;
const RECORDS_CHECKED_FOR_DATE_HEURISTIC = 200;

export const emptyColumnName =
  "__CONVEX_PLACEHOLDER_empty_I23atX0jcndVbFgXoQZffsih7eAqktCyFjgUuAeNBtfr3ySOljPSPSEOPFgprkdBO3zXNiGEJxmJ5ZFPc5C5qKesG80QRPvlJe8vgSxAt9feLTwxTg4PHfVwUaTEJU67FDwldWmTxp1guMPwxQ2jOuhEryTBf3mQ";

// TODO: avoid collisions with fields named "*select".
export const checkboxColumnName = "*select";
export const useStoredColumnOrder = (localStorageKey = "_disabled_") =>
  useLocalStorage<string[]>(`${localStorageKey}_columnOrder`);

export const useStoredHiddenColumns = (localStorageKey = "_disabled_") =>
  useLocalStorage<string[]>(`${localStorageKey}_hiddenColumns`);

export const useDataColumns = ({
  fields,
  localStorageKey = "_disabled_",
  data = [],
  width = 100,
}: {
  tableName: string;
  fields: string[];
  localStorageKey?: string;
  data?: GenericDocument[];
  width?: number;
}) => {
  const [settings] = useGlobalLocalStorage<
    | {
        columnWidths: { [key: string]: number };
      }
    | undefined
  >(localStorageKey, { columnWidths: {} });
  const { columnWidths } = settings || { columnWidths: {} };

  const dateRenderedColumns = useMemo(
    () =>
      data === undefined
        ? []
        : fields.filter((field) => shouldRenderFieldAsDate(field, data)),
    [data, fields],
  );

  const columns = useMemo(
    () =>
      [
        {
          Header: checkboxColumnName,
          minWidth: 40,
          width: 40,
          maxWidth: 40,
          disableResizing: true,
        },
      ].concat(
        fields.map((field) => ({
          Header: field === "" ? emptyColumnName : field,
          accessor: field,
          // id and accessorFn support empty-string key (because there are falsy
          // checks on all these fields and empty string is falsy).
          id: field === "" ? emptyColumnName : undefined,
          accessorFn: (row: any) => row[field],
          isDate: dateRenderedColumns.includes(field),
          minWidth:
            field === "_creationTime"
              ? CREATION_TIME_COLUMN_MIN_WIDTH
              : field === "_id"
                ? ID_COLUMN_WIDTH
                : MIN_COLUMN_WIDTH,
          // Figure out how wide to make each column by default.
          width:
            columnWidths && columnWidths[field]
              ? columnWidths[field]
              : Math.max(
                  (width - DEFAULT_COLUMN_WIDTH) / (fields.length - 1),
                  DEFAULT_COLUMN_WIDTH,
                ),
          disableResizing: false,
          maxWidth: MAX_COLUMN_WIDTH,
        })),
      ),
    // Memoize columns for use with react-table so that new data
    // or other rerender does not reset column widths.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [
      // eslint-disable-next-line react-hooks/exhaustive-deps
      JSON.stringify(columnWidths),
      // eslint-disable-next-line react-hooks/exhaustive-deps
      JSON.stringify(fields),
      // eslint-disable-next-line react-hooks/exhaustive-deps
      JSON.stringify(dateRenderedColumns),
    ],
  );

  if (columns.length) {
    // Set the width of the last column to consume the
    // remainder of available space, if there is any.
    const newWidth =
      width -
      columns
        .slice(0, columns.length - 1)
        .reduce((acc, curr) => acc + curr.width, 0);
    if (newWidth > MIN_COLUMN_WIDTH) {
      columns[columns.length - 1].width = newWidth;
    }
  }
  return columns;
};

function shouldRenderFieldAsDate(field: string, data: GenericDocument[]) {
  if (field === "_creationTime") {
    return true;
  }
  const numChecked = Math.min(data.length, RECORDS_CHECKED_FOR_DATE_HEURISTIC);
  let isDateLike = true;
  let numPopulated = 0;
  for (let i = 0; i < numChecked; i++) {
    const document = data[i];
    if (document[field] === undefined) {
      continue;
    }
    const value = document[field];
    numPopulated += 1;
    if (typeof value !== "number" || !isInCommonUTCTimestampRange(value)) {
      isDateLike = false;
    }
  }
  // If there are no values for this field, assume it's not date-like
  return numPopulated !== 0 && isDateLike;
}
