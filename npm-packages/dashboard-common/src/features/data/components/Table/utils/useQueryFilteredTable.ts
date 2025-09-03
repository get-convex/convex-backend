import { GenericDocument } from "convex/server";
import { useRouter } from "next/router";
import { useMemo, useCallback, useRef, useEffect, useState } from "react";
import { usePaginatedQuery, PaginationStatus } from "convex/react";
import udfs from "@common/udfs";
import { useCounter, useIdle, usePrevious } from "react-use";
import { isFilterValidationError } from "system-udfs/convex/_system/frontend/lib/filters";
import { maximumRowsRead } from "system-udfs/convex/_system/paginationLimits";
import { useNents } from "@common/lib/useNents";

export const pageSize = 25;
const dataPageInactivityTimeMinutes = 1;

// Declared outside of the hook to be referentially stable without useMemo
const dataOnError: GenericDocument[] = [];

export const useQueryFilteredTable = (tableName: string) => {
  const router = useRouter();

  const filters = (router.query.filters as string) || null;

  const isPaused = useIdle(dataPageInactivityTimeMinutes * 1000 * 60, false);

  const { selectedNent } = useNents();
  const { results, loadMore, isLoading, status } = usePaginatedQuery(
    udfs.paginatedTableDocuments.default,
    isPaused
      ? "skip"
      : { table: tableName, filters, componentId: selectedNent?.id ?? null },
    { initialNumItems: pageSize },
  );

  const [everHadResults, setEverHadResults] = useState(false);
  if (!everHadResults && status !== "LoadingFirstPage") {
    setEverHadResults(true);
  }

  const {
    value: maybeStaleResults,
    hasFilters: maybeStaleHasFilters,
    staleAsOf,
  } = useLastKnownValue(results, status, filters);

  const errors = useMemo(
    () => results.filter(isFilterValidationError),
    [results],
  );
  const data =
    errors.length > 0 ? dataOnError : (maybeStaleResults as GenericDocument[]);

  const [
    numRowsReadEstimate,
    { inc: incNumRowsReadEstimate, set: setNumRowsReadEstimate },
  ] = useCounter(0);

  const loadNextPage = useCallback(() => {
    if (status === "CanLoadMore") {
      loadMore(pageSize);
      incNumRowsReadEstimate(maximumRowsRead);
    }
  }, [status, loadMore, incNumRowsReadEstimate]);

  useEffect(() => {
    if (staleAsOf) {
      loadNextPage();
    }
    if (status === "LoadingFirstPage") {
      setNumRowsReadEstimate(0);
    }
  }, [staleAsOf, status, loadNextPage, setNumRowsReadEstimate]);

  return {
    status:
      status === "LoadingFirstPage" && everHadResults ? "Loading" : status,
    loadNextPage,
    isLoading,
    staleAsOf,
    isUsingFilters: maybeStaleHasFilters,
    data,
    errors,
    numRowsReadEstimate,
    isPaused,
  };
};

/**
 * Return the value from the last time this hook received a status that wasn't
 * "LoadingFirstPage".
 *
 * This is used to show old results briefly when users change their filters and
 * the new results are still loading.
 */
function useLastKnownValue<T>(
  value: T[],
  status: PaginationStatus,
  filters: string | null,
) {
  const prevFilters = usePrevious(filters);
  const ref = useRef<{ value: T[]; hasFilters: boolean; staleAsOf: number }>({
    value: [],
    hasFilters: filters !== null,
    staleAsOf: 0,
  });
  ref.current.staleAsOf =
    (prevFilters === filters && ref.current.staleAsOf) || Date.now();
  if (value.length > 0 || status === "Exhausted") {
    ref.current = { value, hasFilters: filters !== null, staleAsOf: 0 };
  }
  return { ...ref.current };
}
