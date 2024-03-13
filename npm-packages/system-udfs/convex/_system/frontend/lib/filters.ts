import { JSONValue, ValidatorJSON, Value, jsonToConvex } from "convex/values";
import type {
  GenericDocument,
  Index,
  SearchIndex,
  VectorIndex,
} from "convex/server";
import * as IdEncoding from "id-encoding";

import { ZodLiteral, z } from "zod";
import isEqual from "lodash/isEqual";

export interface FilterExpression {
  // In the future, this can be extended to support nested clauses.
  clauses: Filter[];
}

export type FilterCommon = {
  id?: string;
  field?: string;
};

export type FilterByBuiltin = {
  op: "eq" | "neq" | "gte" | "lte" | "gt" | "lt";
  value?: JSONValue;
};

export type ValidFilterByBuiltin = {
  [P in keyof FilterCommon]-?: FilterCommon[P];
} & { [P in keyof FilterByBuiltin]-?: FilterByBuiltin[P] };

export type FilterByOr = {
  op: "anyOf" | "noneOf";
  value?: JSONValue[];
};

export type ValidFilterByOr = {
  [P in keyof FilterCommon]-?: FilterCommon[P];
} & { [P in keyof FilterByOr]-?: FilterByOr[P] };

export type ValidFilterByBuiltInOrOr = ValidFilterByBuiltin | ValidFilterByOr;

const TypeFilterValueKeys = [
  "string",
  "boolean",
  "number",
  "bigint",
  "null",
  "id",
  "array",
  "object",
  "bytes",
  "unset",
] as const;

export type TypeFilterValue = (typeof TypeFilterValueKeys)[number];

export type FilterByType = {
  op: TypeFilterOp;
  value?: TypeFilterValue;
};

export type ValidFilterByType = {
  [P in keyof FilterCommon]-?: FilterCommon[P];
} & { [P in keyof FilterByBuiltin]-?: FilterByType[P] };

const TypeFilterOpKeys = ["type", "notype"] as const;

export type TypeFilterOp = (typeof TypeFilterOpKeys)[number];

const objectSuperTypes: TypeFilterValue[] = ["null", "array", "id", "bytes"];

export const isTypeFilterOp = (op: Filter["op"]): op is TypeFilterOp => {
  return TypeFilterOpKeys.includes(op as TypeFilterOp);
};

export type Filter = FilterCommon &
  (FilterByBuiltin | FilterByOr | FilterByType);

const TypeFilterSchema = TypeFilterValueKeys.map<ZodLiteral<string>>((k) =>
  z.literal(k),
);

const FilterSchema = z.array(
  z.union([
    z.object({
      op: z.union([
        z.literal("eq"),
        z.literal("neq"),
        z.literal("gte"),
        z.literal("lte"),
        z.literal("gt"),
        z.literal("lt"),
      ]),
      field: z.string().optional(),
      value: z.any().optional(),
      id: z.string().optional(),
    }),
    z.object({
      op: z.union([z.literal("type"), z.literal("notype")]),
      field: z.string().optional(),
      // @ts-expect-error I don't know how to fix this type error,
      // but i'll test to make sure this works.
      value: z.union(TypeFilterSchema).optional(),
    }),
    z.object({
      op: z.union([z.literal("anyOf"), z.literal("noneOf")]),
      field: z.string().optional(),
      value: z.array(z.any()).optional(),
    }),
  ]),
);

export const FilterExpressionSchema: z.ZodType<FilterExpression> = z.lazy(() =>
  z.object({
    op: z.literal("and").optional(),
    clauses: FilterSchema,
  }),
);

export type ValidFilter =
  | ValidFilterByBuiltin
  | ValidFilterByOr
  | ValidFilterByType;

export const isValidFilter = (f: Filter): f is ValidFilter =>
  f.field !== undefined && f.value !== undefined;

const isType = (value: Value, type: TypeFilterValue): boolean => {
  switch (type) {
    case "null":
      return value === null;
    case "array":
      return Array.isArray(value);
    case "id":
      return typeof value === "string" && IdEncoding.isId(value);
    case "bytes":
      return value instanceof ArrayBuffer;
    case "object":
      return (
        typeof value === "object" &&
        !objectSuperTypes.some((t) => isType(value, t))
      );
    case "unset":
      return value === undefined;
    default:
      return typeof value === type;
  }
};

export const typeOf = (value: Value | undefined): TypeFilterValue => {
  if (value === null) {
    return "null";
  } else if (Array.isArray(value)) {
    return "array";
  } else if (value instanceof ArrayBuffer) {
    return "bytes";
  } else if (typeof value === "object") {
    return "object";
  } else if (value === undefined) {
    return "unset";
  } else {
    const t = typeof value;
    if (t === "symbol" || t === "function" || t === "undefined") {
      throw new Error(`unexpected type of value ${t}`);
    }
    return t;
  }
};

export function applyTypeFilters(
  page: GenericDocument[],
  filters: FilterByType[],
) {
  const validatedFilters = filters.filter<ValidFilterByType>(
    (f): f is ValidFilterByType => isValidFilter(f),
  );
  return page.filter((doc) => {
    for (const filter of validatedFilters) {
      const value = doc[filter.field];
      const matchesType = isType(value, filter.value);
      if (
        (filter.op === "type" && !matchesType) ||
        (filter.op === "notype" && matchesType)
      ) {
        return false;
      }
    }
    return true;
  });
}

export type FilterValidationError = {
  error: string;
  filter: number;
};

export const isFilterValidationError = (
  result: GenericDocument | FilterValidationError,
): result is FilterValidationError => !("_id" in result);

export async function findErrorsInFilters(
  parsedFilters: FilterExpression,
): Promise<FilterValidationError[]> {
  const errors = [];
  for (const [i, filter] of parsedFilters.clauses.entries()) {
    if (filter.value) {
      try {
        jsonToConvex(filter.value);
      } catch (e) {
        errors.push({
          filter: i,
          error: `Invalid value: ${JSON.stringify(filter.value)}.`,
        });
        continue;
      }
    }
  }
  return errors;
}

export function partitionFiltersByOperator(
  filters?: Filter[],
): [ValidFilterByBuiltInOrOr[], ValidFilterByType[]] {
  const builtinFilters =
    filters?.filter<ValidFilterByBuiltin | ValidFilterByOr>(
      (f): f is ValidFilterByBuiltin | ValidFilterByOr => !isTypeFilterOp(f.op),
    ) || [];

  const typeFilters =
    filters?.filter<ValidFilterByType>((f): f is ValidFilterByType =>
      isTypeFilterOp(f.op),
    ) || [];

  return [builtinFilters, typeFilters];
}

// Finds the best index to use for a set of filters and splits the list of filters
// into index filters and non-index filters
export function partitionFiltersByIndexes(
  filters: ValidFilterByBuiltInOrOr[],
  indexes: Index[],
): [
  string | undefined,
  ValidFilterByBuiltInOrOr[],
  ValidFilterByBuiltInOrOr[],
] {
  let selectedIndex: Index | undefined;
  let indexFilters: ValidFilterByBuiltInOrOr[] = [];
  let unindexableFilters: ValidFilterByBuiltInOrOr[] = [];

  const indexableFilters = filters;

  // Start by finding all the filters that are eq operations at the start of the list of filters
  // There's probably a better way to do this, but this code should be temporary until we support
  // more complex indexable filters
  const indexableFiltersWithEqOp: ValidFilterByBuiltInOrOr[] = [];
  for (const indexableFilter of indexableFilters) {
    if (indexableFilter.op === "eq") {
      indexableFiltersWithEqOp.push(indexableFilter);
    } else {
      break;
    }
  }

  // Worst-case time complexity: O(indexableFiltersWithEqOp.length * indexes.length)
  // This outer loop is to find the longest subset of indexable filters that match an index
  for (
    let indexedFieldsCount = indexableFiltersWithEqOp.length;
    indexedFieldsCount > 0;
    indexedFieldsCount--
  ) {
    const filteredFields = indexableFiltersWithEqOp
      .slice(0, indexedFieldsCount)
      .map((f) => f.field);
    // This inner loop is to find the first index subset that matches the subset of indexable filters
    for (const index of indexes) {
      const doesIndexSubsetMatchIndexableFields = isEqual(
        index.fields.slice(0, indexedFieldsCount),
        filteredFields,
      );
      if (doesIndexSubsetMatchIndexableFields) {
        // We found the best index to use and which fields to use it with!
        selectedIndex = index;
        indexFilters = indexableFiltersWithEqOp.slice(0, indexedFieldsCount);
        unindexableFilters = filters.slice(indexFilters.length);
        break;
      }
    }
    if (selectedIndex !== undefined) {
      break;
    }
  }

  if (!selectedIndex) {
    unindexableFilters = filters;
  }

  return [selectedIndex?.indexDescriptor, indexFilters, unindexableFilters];
}

export function parseAndFilterToSingleTable(
  tableName: string,
  schema: any,
): undefined | SchemaJson {
  if (!schema) {
    return undefined;
  }
  const result = JSON.parse(schema) as SchemaJson;
  result.tables = result.tables.filter(
    (table) => table.tableName === tableName,
  );
  return result;
}
type TableDefinition = {
  tableName: string;
  indexes: Index[];
  searchIndexes: SearchIndex[];
  vectorIndexes?: VectorIndex[];
  documentType: ValidatorJSON;
};

export type SchemaJson = {
  tables: TableDefinition[];
  schemaValidation: boolean;
};
