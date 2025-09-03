import { decode, encodeURI, isValid } from "js-base64";
import { useRouter } from "next/router";
import { createGlobalState, usePrevious } from "react-use";

import { useEffect, useContext } from "react";
import {
  DatabaseFilterExpression,
  FilterExpressionSchema,
  isValidFilter,
} from "system-udfs/convex/_system/frontend/lib/filters";
import isEqual from "lodash/isEqual";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

// Global state keeping track of filters for all tables.
export const useFilterMap = createGlobalState(
  {} as Record<string, DatabaseFilterExpression | undefined>,
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

    function populateQueryFilters(newFilters: DatabaseFilterExpression) {
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
    let f: DatabaseFilterExpression;
    try {
      f = JSON.parse(decodedFilters);
      FilterExpressionSchema.parse(f);
    } catch (e) {
      // The filters decoded from b64, but failed to parse.
      deleteQueryFilters();
      return;
    }

    // No clauses in the filters, lets clear out the query param.
    if (f.clauses.length === 0 && (!f.index || f.index.clauses.length === 0)) {
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
    applyFiltersWithHistory: async (newFilters?: DatabaseFilterExpression) => {
      if (newFilters) {
        const newFilterMap = { ...filterMap, [tableName]: newFilters };
        if (
          !newFilterMap[tableName] ||
          (newFilterMap[tableName]?.clauses.length === 0 &&
            !newFilterMap[tableName]?.index &&
            newFilterMap[tableName]?.order === undefined)
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
    hasFilters: hasValidEnabledFilters(filterMap[tableName]),
  };
};

function hasValidEnabledFilters(filters?: DatabaseFilterExpression) {
  return (
    !!filters &&
    (filters.clauses.filter(isValidFilter).filter((f) => f.enabled !== false)
      .length > 0 ||
      (filters.index?.clauses.filter((f) => f.enabled).length ?? 0) > 0)
  );
}

export function areAllFiltersValid(filters?: DatabaseFilterExpression) {
  return filters === undefined || filters.clauses.every(isValidFilter);
}

export function useFilterHistory(
  tableName: string,
  componentId: string | null,
): {
  filterHistory: DatabaseFilterExpression[];
  appendFilterHistory: (value: DatabaseFilterExpression) => void;
} {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  const [filterHistory, setFilterHistory] = useGlobalLocalStorage(
    `filterHistory/${deployment?.name}/${componentId ? `${componentId}/` : ""}${tableName}`,
    [] as DatabaseFilterExpression[],
  );

  return {
    filterHistory,
    appendFilterHistory: (value) => {
      setFilterHistory((prev: DatabaseFilterExpression[]) => {
        if (
          (prev.length > 0 && isEqual(prev[0], value)) ||
          (value.clauses.length === 0 &&
            (!value.index || value.index.clauses.length === 0))
        ) {
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
