import { useCallback, useEffect, useMemo } from "react";
import { NextRouter, useRouter } from "next/router";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { useNents } from "@common/lib/useNents";
import { isUserTableName } from "@common/lib/utils";

export type TableMetadata = {
  name: string | null;
  tableNames: string[];
  selectTable: (table: string) => void;
};

export function useTableMetadata(): TableMetadata | undefined {
  const router = useRouter();
  const { selectedNent } = useNents();

  const tableMapping = useQuery(udfs.getTableMapping.default, {
    componentId: selectedNent?.id ?? null,
  });

  const tableNames = useMemo(
    () =>
      tableMapping === undefined
        ? undefined
        : Object.values(tableMapping).filter(isUserTableName).sort(),
    [tableMapping],
  );

  const selectTable = useCallback(
    (table: string) => {
      if (tableNames === undefined) {
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
    [tableNames, router],
  );

  if (tableNames === undefined) {
    return undefined;
  }

  const tableNameFromURL = router.query.table as string | undefined;
  const firstTableName = (tableNames[0] as string | undefined) ?? null;

  const shownTableName =
    tableNameFromURL !== undefined && tableNames.includes(tableNameFromURL)
      ? tableNameFromURL
      : firstTableName;

  return {
    name: shownTableName,
    tableNames,
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

export function shallowNavigate(
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
