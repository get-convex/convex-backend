import {
  DatabaseIndexFilter,
  FilterByIndex,
  FilterByIndexRange,
  FilterExpression,
} from "system-udfs/convex/_system/frontend/lib/filters";

export function clearFilters(
  filters: FilterExpression | undefined,
): FilterExpression {
  return {
    clauses: [],
    index:
      !filters?.index ||
      // When clearing a search filter, we go back to the default index.
      // We do so because we assume that when the user clears filters,
      // they want to see all documents, and an empty search filter would
      // show none. This is a little bit incoherent with the behavior
      // for database filters (which clears all filters but stays on the
      // same index).
      "search" in filters.index
        ? undefined
        : ({
            name: filters.index.name,
            clauses: (
              filters.index.clauses satisfies
                | FilterByIndex[]
                | [...FilterByIndex[], FilterByIndexRange]
            ).map((clause) => ({
              ...clause,
              enabled: false,
            })) as FilterByIndex[] | [...FilterByIndex[], FilterByIndexRange],
          } satisfies DatabaseIndexFilter),
    order: filters?.order,
  };
}
