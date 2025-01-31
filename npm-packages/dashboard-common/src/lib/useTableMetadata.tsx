import { useCallback, useEffect } from "react";
import { Shape } from "shapes";
import { NextRouter, useRouter } from "next/router";
import { useTableShapes } from "lib/deploymentApi";

export type TableMetadata = {
  name: string | null;
  tables: Map<string, Shape>;
  selectTable: (table: string) => void;
};

export function useTableMetadata(): TableMetadata | undefined {
  const router = useRouter();

  // Gets initial Table Shapes (names, columns)
  const { tables } = useTableShapes();

  const selectTable = useCallback(
    (table: string) => {
      if (tables === undefined) {
        return;
      }
      if (table === router.query.table) {
        return;
      }
      void shallowNavigate(router, {
        ...router.query,
        table,
        filters: undefined,
      });
    },
    [tables, router],
  );

  if (tables === undefined) {
    return undefined;
  }

  const tableNameFromURL = router.query.table as string | undefined;
  const tableNames = Array.from(tables.keys());
  const firstTableName = (tableNames[0] as string | undefined) ?? null;

  const shownTableName =
    tableNameFromURL !== undefined && tableNames.includes(tableNameFromURL)
      ? tableNameFromURL
      : firstTableName;

  return {
    name: shownTableName,
    tables,
    selectTable,
  };
}

export function useTableMetadataAndUpdateURL(): TableMetadata | undefined {
  const tableMetadata = useTableMetadata();

  const router = useRouter();

  const tablesLoaded = tableMetadata !== undefined;
  const selectedTableName = tableMetadata?.name;

  // Remove a stale table name from the URL
  useEffect(() => {
    if (
      tablesLoaded &&
      selectedTableName !== null &&
      router.query.table !== selectedTableName
    ) {
      const table = selectedTableName ?? undefined;
      void shallowNavigate(router, {
        ...router.query,
        table,
        filters: undefined,
      });
    }
  }, [router, tablesLoaded, selectedTableName]);

  return tableMetadata;
}

function shallowNavigate(
  router: NextRouter,
  newQuery: Record<string, string | undefined>,
) {
  return router.replace(
    // Filter out undefineds so they don't show up as empty strings in URL
    { query: objectWithoutUndefinedValues(newQuery) },
    undefined,
    { shallow: true },
  );
}

function objectWithoutUndefinedValues<T extends Record<string, any>>(
  obj: T,
): T {
  return Object.keys(obj).reduce(
    (acc, key) => (obj[key] === undefined ? acc : { ...acc, [key]: obj[key] }),
    {} as T,
  );
}
