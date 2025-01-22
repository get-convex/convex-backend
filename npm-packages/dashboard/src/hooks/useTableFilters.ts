import { decode, encodeURI, isValid } from "js-base64";
import { useRouter } from "next/router";
import { createGlobalState, usePrevious } from "react-use";

import { useEffect } from "react";
import {
  FilterExpression,
  FilterExpressionSchema,
  isValidFilter,
} from "system-udfs/convex/_system/frontend/lib/filters";
import isEqual from "lodash/isEqual";
import { useGlobalLocalStorage } from "dashboard-common";

// Global state keeping track of filters for all tables.
export const useFilterMap = createGlobalState(
  {} as Record<string, FilterExpression | undefined>,
);

const useInitializeFilters = (
  tableName: string,
  componentId: string | null,
) => {
  const { query, replace, isReady: isRouterReady } = useRouter();
  const [filterMap, setFilterMap] = useFilterMap();
  const prevTableName = usePrevious(tableName);
  const { appendFilterHistory } = useFilterHistory(tableName, componentId);

  // Effect to initialize filters on mount.
  useEffect(() => {
    // This hook should only run if the router is ready or if the selected table changed.
    if (!isRouterReady || prevTableName === tableName) {
      return;
    }

    function deleteQueryFilters() {
      if (!query.filters) {
        return;
      }
      delete query.filters;
      void replace({ query }, undefined, {
        shallow: true,
      });
      setFilterMap({ ...filterMap, [tableName]: undefined });
    }

    function populateQueryFilters(newFilters: FilterExpression) {
      query.filters = encodeURI(JSON.stringify(newFilters));
      void replace(
        {
          query,
        },
        undefined,
        { shallow: true },
      );
    }

    // We're mounting useTableFilters and already have filters from a previous mount.
    const storedFilters = filterMap[tableName];
    if (storedFilters && !query.filters) {
      populateQueryFilters(storedFilters);
      return;
    }

    if (!isValid(query.filters as string)) {
      deleteQueryFilters();
      return;
    }

    const decodedFilters = decode(query.filters as string);
    let f: FilterExpression;
    try {
      f = JSON.parse(decodedFilters);
      FilterExpressionSchema.parse(f);
    } catch (e) {
      // The filters decoded from b64, but failed to parse.
      deleteQueryFilters();
      return;
    }

    // No clauses in the filters, lets clear out the query param.
    if (f.clauses.length === 0) {
      deleteQueryFilters();
      return;
    }

    // Found filters in the query, store in global state.
    appendFilterHistory(f);
    setFilterMap({ ...filterMap, [tableName]: f });
  }, [
    appendFilterHistory,
    filterMap,
    isRouterReady,
    prevTableName,
    query,
    replace,
    setFilterMap,
    tableName,
  ]);
};

export const useTableFilters = (
  tableName: string,
  componentId: string | null,
) => {
  const { query, replace } = useRouter();
  const [filterMap, setFilterMap] = useFilterMap();
  const { appendFilterHistory } = useFilterHistory(tableName, componentId);

  useInitializeFilters(tableName, componentId);

  return {
    filters: filterMap[tableName],
    // Make sure a new object is created so the hook is re-rendered
    changeFilters: async (newFilters?: FilterExpression) => {
      if (newFilters) {
        const newFilterMap = { ...filterMap, [tableName]: newFilters };
        if (
          !newFilterMap[tableName] ||
          newFilterMap[tableName]?.clauses.length === 0
        ) {
          delete query.filters;
        } else {
          query.filters = encodeURI(JSON.stringify(newFilterMap[tableName]));
        }
        setFilterMap(newFilterMap);
        appendFilterHistory(newFilters);
        await replace(
          {
            query,
          },
          undefined,
          { shallow: true },
        );
      }
    },
    hasFilters: hasValidFilters(filterMap[tableName]),
  };
};

function hasValidFilters(filters?: FilterExpression) {
  return !!filters && filters.clauses.filter(isValidFilter).length > 0;
}

export function areAllFiltersValid(filters?: FilterExpression) {
  return filters === undefined || filters.clauses.every(isValidFilter);
}

export function useFilterHistory(
  tableName: string,
  componentId: string | null,
): {
  filterHistory: FilterExpression[];
  appendFilterHistory: (value: FilterExpression) => void;
} {
  const [filterHistory, setFilterHistory] = useGlobalLocalStorage(
    `filterHistory/${componentId ? `${componentId}/` : ""}${tableName}`,
    [] as FilterExpression[],
  );

  return {
    filterHistory,
    appendFilterHistory: (value) => {
      setFilterHistory((prev: FilterExpression[]) => {
        if (prev.length > 0 && isEqual(prev[0], value)) {
          return prev;
        }
        const updatedHistory = [value, ...prev];
        if (updatedHistory.length > 25) {
          updatedHistory.pop();
        }
        return updatedHistory;
      });
    },
  };
}
