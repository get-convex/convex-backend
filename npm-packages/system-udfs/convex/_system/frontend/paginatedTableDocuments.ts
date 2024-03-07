import { Index, OrderedQuery } from "convex/server";
import { decode } from "js-base64";
import {
  Expression,
  ExpressionOrValue,
  FilterBuilder,
  GenericDocument,
  GenericTableInfo,
  PaginationResult,
} from "convex/server";
import {
  FilterByBuiltin,
  FilterByOr,
  FilterExpression,
  FilterExpressionSchema,
  FilterValidationError,
  ValidFilterByBuiltInOrOr,
  ValidFilterByBuiltin,
  ValidFilterByOr,
  applyTypeFilters,
  findErrorsInFilters,
  isValidFilter,
  parseAndFilterToSingleTable,
  partitionFiltersByIndexes,
  partitionFiltersByOperator,
} from "./lib/filters";
import { Value, jsonToConvex } from "convex/values";
import { queryGeneric } from "../secretSystemTables";
import { getSchemaByState } from "./getSchemas";
import { maximumBytesRead, maximumRowsRead } from "../paginationLimits";
import { paginationOptsValidator } from "convex/server";
import { v } from "convex/values";

export default queryGeneric({
  args: {
    paginationOpts: paginationOptsValidator,
    table: v.string(),
    filters: v.union(v.string(), v.null()),
  },
  /**
   * @param paginationOpts - Pagination options
   * @param table - The name of the table to query
   * @param filters - The expected value of filters is a b64-encoded string {@link FilterExpression}.
   *  {@link FilterExpression} contains {@link JSONValue}s so that invalid IDs being filtered are still able to
   *  be passed into the UDF over the wire. Since {@link JSONValue} type contains keys beginning with dollar signs, we then encode it to
   *  a string format so it can be passed over the wire as well.
   * @returns A paginated list of documents matching the provided filters
   */
  handler: async (
    { db },
    { paginationOpts, table, filters },
  ): Promise<PaginationResult<GenericDocument | FilterValidationError>> => {
    const parsedFilters: FilterExpression | null = filters
      ? (JSON.parse(decode(filters)) as FilterExpression)
      : null;

    // This will throw an error if parsedFilters does not match the filter expression schema,
    // which should only happens if a dashboard user manually edits the `filters` query parameter
    // the dashboard should not allow this to happen by deleting the query parameter if it is invalid.
    parsedFilters && FilterExpressionSchema.parse(parsedFilters);

    if (parsedFilters && parsedFilters.clauses?.length) {
      const errors = await findErrorsInFilters(parsedFilters);
      if (errors.length) {
        // Hack to trick usePaginatedQuery that we've actually called `paginate`, but really returning a list of
        // error-describing objects
        return Promise.resolve({
          page: errors,
          isDone: true,
          continueCursor: "",
        });
      }
    }

    const [builtinFilters, typeFilters] = partitionFiltersByOperator(
      parsedFilters?.clauses,
    );

    const queryInitializer = db.query(table);
    let query: OrderedQuery<any> | undefined = undefined;

    if (builtinFilters !== undefined && builtinFilters.length > 0) {
      // Edge case: requesting a single ID - use a db.get
      const isAFilterForSingleDocument =
        builtinFilters.length === 1 &&
        "field" in builtinFilters[0] &&
        builtinFilters[0].field === "_id" &&
        builtinFilters[0].op === "eq";

      if (isAFilterForSingleDocument) {
        const emptyResult = {
          page: [],
          isDone: true,
          continueCursor: "",
        };
        const documentId = builtinFilters[0].value;
        if (typeof documentId !== "string") {
          return emptyResult;
        }
        const normalizedId = db.normalizeId(table, documentId);
        if (normalizedId === null) {
          return emptyResult;
        }
        const document = await db.get(normalizedId);
        if (document === null) {
          return emptyResult;
        }
        return {
          page: [document],
          isDone: true,
          continueCursor: "",
        };
      }

      // Let's find out if we can use an index from the schema.
      const schemaData = await getSchemaByState(
        (db as any).privateSystem,
        "active",
      );
      const indexes: Index[] = schemaData?.schema
        ? parseAndFilterToSingleTable(table, schemaData.schema)?.tables[0]
            ?.indexes || []
        : [];

      const [selectedIndex, indexFilters, builtInFiltersAfterIndex] =
        partitionFiltersByIndexes(builtinFilters, indexes);

      if (selectedIndex && indexFilters.length > 0) {
        query = queryInitializer
          .withIndex(selectedIndex, (q) => applyIndexFilters(q, indexFilters))
          .order("desc");
      }

      query = (query || queryInitializer.order("desc")).filter((q) =>
        applyBuiltinFilters(q, builtInFiltersAfterIndex),
      );
    }

    const internalPaginateOpts = {
      ...paginationOpts,
      // these are internal options
      maximumRowsRead,
      maximumBytesRead,
    };

    const { page, ...rest } = await (
      query || queryInitializer.order("desc")
    ).paginate(internalPaginateOpts);

    const filteredPage = typeFilters
      ? applyTypeFilters(page, typeFilters)
      : page;

    return { page: filteredPage, ...rest };
  },
});

function applyIndexFilters(
  // TODO(CX-5718): Figure out the generic typing here
  q: any,
  indexFilters: ValidFilterByBuiltInOrOr[],
): any {
  if (indexFilters.length === 0) {
    return q;
  }
  const filter = indexFilters[0];
  const value = jsonToConvex(filter.value);
  if (filter.op !== "eq") {
    throw new Error("applyIndexFilters called with non-equals operator");
  }
  return applyIndexFilters(q.eq(filter.field, value), indexFilters.slice(1));
}

// Applies built in filters to the query.
function applyBuiltinFilters(
  q: FilterBuilder<GenericTableInfo>,
  filters: (FilterByBuiltin | FilterByOr)[],
): ExpressionOrValue<boolean> {
  const validatedFilters = filters.filter<
    ValidFilterByBuiltin | ValidFilterByOr
  >((f): f is ValidFilterByBuiltin | ValidFilterByOr => isValidFilter(f));
  // If there are no valid filters, return the query unchanged.
  if (validatedFilters.length === 0) {
    return true;
  }
  return q.and(
    ...validatedFilters.map((f) => {
      const value = jsonToConvex(f.value);

      if (f.op === "anyOf" || f.op === "noneOf") {
        if (f.value.length === 0) {
          // Even though the filter is valid, don't apply the operation if there are not values to compare to.
          return true;
        }
        if (f.op === "anyOf") {
          return q.or(
            ...f.value.map((v) => q.eq(q.field(f.field), jsonToConvex(v))),
          );
        } else {
          return q.and(
            ...f.value.map((v) => q.neq(q.field(f.field), jsonToConvex(v))),
          );
        }
      }

      // q.eq and q.neq support undefined, while q.lt, q.gt, etc. do not, so we have to cast.
      const comparison = q[f.op] as (
        f: Expression<Value>,
        v: ExpressionOrValue<Value>,
      ) => Expression<boolean>;
      return comparison(q.field(f.field), value);
    }),
  );
}
