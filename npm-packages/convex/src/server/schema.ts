/**
 * Utilities for defining the schema of your Convex project.
 *
 * ## Usage
 *
 * Schemas should be placed in a `schema.ts` file in your `convex/` directory.
 *
 * Schema definitions should be built using {@link defineSchema},
 * {@link defineTable}, and {@link values.v}. Make sure to export the schema as the
 * default export.
 *
 * ```ts
 * import { defineSchema, defineTable } from "convex/server";
 * import { v } from "convex/values";
 *
 *  export default defineSchema({
 *    messages: defineTable({
 *      body: v.string(),
 *      user: v.id("users"),
 *    }),
 *    users: defineTable({
 *      name: v.string(),
 *    }),
 *  });
 * ```
 *
 * To learn more about schemas, see [Defining a Schema](https://docs.convex.dev/using/schemas).
 * @module
 */
import {
  AnyDataModel,
  GenericDataModel,
  GenericTableIndexes,
  GenericTableSearchIndexes,
  GenericTableVectorIndexes,
  TableNamesInDataModel,
} from "../server/data_model.js";
import {
  IdField,
  IndexTiebreakerField,
  SystemFields,
  SystemIndexes,
} from "../server/system_fields.js";
import { Expand } from "../type_utils.js";
import {
  GenericValidator,
  ObjectType,
  isValidator,
  v,
} from "../values/validator.js";
import { VObject, Validator } from "../values/validators.js";

/**
 * Extract all of the index field paths within a {@link Validator}.
 *
 * This is used within {@link defineTable}.
 * @public
 */
type ExtractFieldPaths<T extends Validator<any, any, any>> =
  // Add in the system fields available in index definitions.
  // This should be everything except for `_id` because thats added to indexes
  // automatically.
  T["fieldPaths"] | keyof SystemFields;

/**
 * Extract the {@link GenericDocument} within a {@link Validator} and
 * add on the system fields.
 *
 * This is used within {@link defineTable}.
 * @public
 */
type ExtractDocument<T extends Validator<any, any, any>> =
  // Add the system fields to `Value` (except `_id` because it depends on
  //the table name) and trick TypeScript into expanding them.
  Expand<SystemFields & T["type"]>;

export interface DbIndexConfig<
  FirstFieldPath extends string,
  RestFieldPaths extends string[],
> {
  /**
   * The fields to index, in order. Must specify at least one field.
   */
  fields: [FirstFieldPath, ...RestFieldPaths];
}

/**
 * The configuration for a full text search index.
 *
 * @public
 */
export interface SearchIndexConfig<
  SearchField extends string,
  FilterFields extends string,
> {
  /**
   * The field to index for full text search.
   *
   * This must be a field of type `string`.
   */
  searchField: SearchField;

  /**
   * Additional fields to index for fast filtering when running search queries.
   */
  filterFields?: FilterFields[];
}

/**
 * The configuration for a vector index.
 *
 * @public
 */
export interface VectorIndexConfig<
  VectorField extends string,
  FilterFields extends string,
> {
  /**
   * The field to index for vector search.
   *
   * This must be a field of type `v.array(v.float64())` (or a union)
   */
  vectorField: VectorField;
  /**
   * The length of the vectors indexed. This must be between 2 and 2048 inclusive.
   */
  dimensions: number;
  /**
   * Additional fields to index for fast filtering when running vector searches.
   */
  filterFields?: FilterFields[];
}

/**
 * @internal
 */
export type VectorIndex = {
  indexDescriptor: string;
  vectorField: string;
  dimensions: number;
  filterFields: string[];
};

/**
 * @internal
 */
export type Index = {
  indexDescriptor: string;
  fields: string[];
};

/**
 * @internal
 */
export type SearchIndex = {
  indexDescriptor: string;
  searchField: string;
  filterFields: string[];
};

/**
 * The aggregation types supported by FlowFields.
 * @public
 */
export type FlowFieldAggregationType =
  | "count"
  | "sum"
  | "avg"
  | "min"
  | "max"
  | "lookup"
  | "exist";

/**
 * A FlowField expression — recursive type for ComputedField `expr` values.
 * Generic over `Fields` to provide autocomplete for field references.
 * @public
 */
export type FlowExpr<Fields extends string = string> =
  | `$${Fields}` // field references — IDE suggests $name, $totalSpent, etc.
  | (string & {}) // string literals like "VIP" — allows any string without killing autocomplete
  | number
  | boolean
  | null
  | FlowExprAdd<Fields>
  | FlowExprSub<Fields>
  | FlowExprMul<Fields>
  | FlowExprDiv<Fields>
  | FlowExprGt<Fields>
  | FlowExprGte<Fields>
  | FlowExprLt<Fields>
  | FlowExprLte<Fields>
  | FlowExprEq<Fields>
  | FlowExprNe<Fields>
  | FlowExprCond<Fields>
  | FlowExprConcat<Fields>
  | FlowExprIfNull<Fields>;

/** @public */
export interface FlowExprAdd<F extends string = string> {
  $add: [FlowExpr<F>, FlowExpr<F>];
}
/** @public */
export interface FlowExprSub<F extends string = string> {
  $sub: [FlowExpr<F>, FlowExpr<F>];
}
/** @public */
export interface FlowExprMul<F extends string = string> {
  $mul: [FlowExpr<F>, FlowExpr<F>];
}
/** @public */
export interface FlowExprDiv<F extends string = string> {
  $div: [FlowExpr<F>, FlowExpr<F>];
}
/** @public */
export interface FlowExprGt<F extends string = string> {
  $gt: [FlowExpr<F>, FlowExpr<F>];
}
/** @public */
export interface FlowExprGte<F extends string = string> {
  $gte: [FlowExpr<F>, FlowExpr<F>];
}
/** @public */
export interface FlowExprLt<F extends string = string> {
  $lt: [FlowExpr<F>, FlowExpr<F>];
}
/** @public */
export interface FlowExprLte<F extends string = string> {
  $lte: [FlowExpr<F>, FlowExpr<F>];
}
/** @public */
export interface FlowExprEq<F extends string = string> {
  $eq: [FlowExpr<F>, FlowExpr<F>];
}
/** @public */
export interface FlowExprNe<F extends string = string> {
  $ne: [FlowExpr<F>, FlowExpr<F>];
}
/** @public */
export interface FlowExprCond<F extends string = string> {
  $cond: FlowExpr<F>;
  $then: FlowExpr<F>;
  $else: FlowExpr<F>;
}
/** @public */
export interface FlowExprConcat<F extends string = string> {
  $concat: FlowExpr<F>[];
}
/** @public */
export interface FlowExprIfNull<F extends string = string> {
  $ifNull: [FlowExpr<F>, FlowExpr<F>];
}

/** @public */
export type FlowFieldFilterRef<FilterNames extends string = string> = {
  $field: FilterNames;
};
/** @public */
export type FlowFieldFilterValue<FilterNames extends string = string> =
  | string
  | number
  | boolean
  | null
  | FlowFieldFilterRef<FilterNames>;
/** @public */
export type FlowFieldFilter<FilterNames extends string = string> = Record<
  string,
  FlowFieldFilterValue<FilterNames>
>;

type StringKeysOf<T> = Extract<keyof T, string>;

/**
 * The configuration for a FlowField — a cross-table aggregation resolved at read time.
 *
 * @public
 */
export interface FlowFieldConfig<
  Returns extends Validator<any, any, any> = Validator<any, any, any>,
  FilterNames extends string = string,
  Source extends string = string,
  Key extends string = string,
  Field extends string = string,
> {
  /** The validator describing the return type of this FlowField. */
  returns: Returns;
  /** The aggregation type. */
  type: FlowFieldAggregationType;
  /** The source table to aggregate from. */
  source: Source;
  /** The field on the source table that references this table's `_id`. */
  key: Key;
  /** The field on the source table to aggregate (required for sum/avg/min/max). */
  field?: Field;
  /** Static filter conditions and `{ $field: "flowFilterName" }` references. */
  filter?: FlowFieldFilter<FilterNames>;
}

/**
 * The configuration for a ComputedField — a row-level expression evaluated from stored + FlowField values.
 *
 * @public
 */
export interface ComputedFieldConfig<
  Returns extends Validator<any, any, any> = Validator<any, any, any>,
  Fields extends string = string,
> {
  /** The validator describing the return type of this ComputedField. */
  returns: Returns;
  /** The expression DSL (JSON-serializable). */
  expr: FlowExpr<Fields>;
}

/**
 * The configuration for a FlowFilter — a runtime parameter that parameterizes FlowField aggregations.
 *
 * @public
 */
export interface FlowFilterConfig<
  FilterType extends Validator<any, any, any> = Validator<any, any, any>,
> {
  /** The validator describing the type of this FlowFilter parameter. */
  type: FilterType;
}

/**
 * @internal
 */
export type SerializedFlowField = {
  fieldName: string;
  returns: object;
  aggregation: FlowFieldAggregationType;
  source: string;
  key: string;
  field: string | undefined;
  filter: FlowFieldFilter | undefined;
};

/**
 * @internal
 */
export type SerializedComputedField = {
  fieldName: string;
  returns: object;
  expr: FlowExpr;
};

/**
 * @internal
 */
export type SerializedFlowFilter = {
  fieldName: string;
  filterType: object;
};

/**
 * Options for defining an index.
 *
 * @public
 */
export interface IndexOptions {
  /**
   * Whether the index should be staged.
   *
   * For large tables, index backfill can be slow. Staging an index allows you
   * to push the schema and enable the index later.
   *
   * If `staged` is `true`, the index will be staged and will not be enabled
   * until the staged flag is removed. Staged indexes do not block push
   * completion. Staged indexes cannot be used in queries.
   */
  staged?: boolean;
}

/**
 * The definition of a table within a schema.
 *
 * This should be produced by using {@link defineTable}.
 * @public
 */
export class TableDefinition<
  DocumentType extends Validator<any, any, any> = Validator<any, any, any>,
  Indexes extends GenericTableIndexes = {},
  SearchIndexes extends GenericTableSearchIndexes = {},
  VectorIndexes extends GenericTableVectorIndexes = {},
  FlowFields extends Record<string, any> = {},
  ComputedFields extends Record<string, any> = {},
  FlowFilters extends Record<string, any> = {},
  FlowFieldRefs extends Record<
    string,
    { source: string; key: string; field: string }
  > = {},
> {
  private indexes: Index[];
  private stagedDbIndexes: Index[];
  private searchIndexes: SearchIndex[];
  private stagedSearchIndexes: SearchIndex[];
  private vectorIndexes: VectorIndex[];
  private stagedVectorIndexes: VectorIndex[];
  private flowFieldDefs: SerializedFlowField[];
  private computedFieldDefs: SerializedComputedField[];
  private flowFilterDefs: SerializedFlowFilter[];
  // The type of documents stored in this table.
  validator: DocumentType;

  /**
   * @internal
   */
  constructor(documentType: DocumentType) {
    this.indexes = [];
    this.stagedDbIndexes = [];
    this.searchIndexes = [];
    this.stagedSearchIndexes = [];
    this.vectorIndexes = [];
    this.stagedVectorIndexes = [];
    this.flowFieldDefs = [];
    this.computedFieldDefs = [];
    this.flowFilterDefs = [];
    this.validator = documentType;
  }

  /**
   * This API is experimental: it may change or disappear.
   *
   * Returns indexes defined on this table.
   * Intended for the advanced use cases of dynamically deciding which index to use for a query.
   * If you think you need this, please chime in on ths issue in the Convex JS GitHub repo.
   * https://github.com/get-convex/convex-js/issues/49
   */
  " indexes"(): { indexDescriptor: string; fields: string[] }[] {
    return this.indexes;
  }

  /**
   * Define an index on this table.
   *
   * Indexes speed up queries by allowing efficient lookups on specific fields.
   * Use `.withIndex()` in your queries to leverage them.
   *
   * Index fields must be queried in the same order they are defined. If you
   * need to query by `field2` then `field1`, create a separate index with
   * that field order.
   *
   * @example
   * ```ts
   * defineTable({
   *   userId: v.id("users"),
   *   status: v.string(),
   *   updatedAt: v.number(),
   * })
   *   // Name indexes after their fields:
   *   .index("by_userId", ["userId"])
   *   .index("by_status_updatedAt", ["status", "updatedAt"])
   * ```
   *
   * **Best practice:** Always include all index fields in the index name
   * (e.g., `"by_field1_and_field2"`).
   *
   * @param name - The name of the index.
   * @param indexConfig - The index configuration object.
   * @returns A {@link TableDefinition} with this index included.
   *
   * @see https://docs.convex.dev/database/reading-data/indexes
   */
  index<
    IndexName extends string,
    FirstFieldPath extends ExtractFieldPaths<DocumentType>,
    RestFieldPaths extends ExtractFieldPaths<DocumentType>[],
  >(
    name: IndexName,
    indexConfig: Expand<
      DbIndexConfig<FirstFieldPath, RestFieldPaths> &
        IndexOptions & { staged?: false }
    >,
  ): TableDefinition<
    DocumentType,
    Expand<
      Indexes &
        Record<
          IndexName,
          [FirstFieldPath, ...RestFieldPaths, IndexTiebreakerField]
        >
    >,
    SearchIndexes,
    VectorIndexes,
    FlowFields,
    ComputedFields,
    FlowFilters,
    FlowFieldRefs
  >;

  /**
   * Define an index on this table.
   *
   * To learn about indexes, see [Defining Indexes](https://docs.convex.dev/database/reading-data/indexes).
   *
   * @param name - The name of the index.
   * @param fields - The fields to index, in order. Must specify at least one
   * field.
   * @returns A {@link TableDefinition} with this index included.
   */
  index<
    IndexName extends string,
    FirstFieldPath extends ExtractFieldPaths<DocumentType>,
    RestFieldPaths extends ExtractFieldPaths<DocumentType>[],
  >(
    name: IndexName,
    fields: [FirstFieldPath, ...RestFieldPaths],
  ): TableDefinition<
    DocumentType,
    Expand<
      Indexes &
        Record<
          IndexName,
          [FirstFieldPath, ...RestFieldPaths, IndexTiebreakerField]
        >
    >,
    SearchIndexes,
    VectorIndexes,
    FlowFields,
    ComputedFields,
    FlowFilters,
    FlowFieldRefs
  >;

  /**
   * Define a staged index on this table.
   *
   * For large tables, index backfill can be slow. Staging an index allows you
   * to push the schema and enable the index later.
   *
   * If `staged` is `true`, the index will be staged and will not be enabled
   * until the staged flag is removed. Staged indexes do not block push
   * completion. Staged indexes cannot be used in queries.
   *
   * To learn about indexes, see [Defining Indexes](https://docs.convex.dev/using/indexes).
   *
   * @param name - The name of the index.
   * @param indexConfig - The index configuration object.
   * @returns A {@link TableDefinition} with this index included.
   */
  index<
    IndexName extends string,
    FirstFieldPath extends ExtractFieldPaths<DocumentType>,
    RestFieldPaths extends ExtractFieldPaths<DocumentType>[],
  >(
    name: IndexName,
    indexConfig: Expand<
      DbIndexConfig<FirstFieldPath, RestFieldPaths> &
        IndexOptions & { staged: true }
    >,
  ): TableDefinition<DocumentType, Indexes, SearchIndexes, VectorIndexes, FlowFields, ComputedFields, FlowFilters, FlowFieldRefs>;

  index<
    IndexName extends string,
    FirstFieldPath extends ExtractFieldPaths<DocumentType>,
    RestFieldPaths extends ExtractFieldPaths<DocumentType>[],
  >(
    name: IndexName,
    indexConfig:
      | Expand<DbIndexConfig<FirstFieldPath, RestFieldPaths> & IndexOptions>
      | [FirstFieldPath, ...RestFieldPaths],
  ) {
    if (Array.isArray(indexConfig)) {
      // indexConfig is [FirstFieldPath, ...RestFieldPaths]
      this.indexes.push({
        indexDescriptor: name,
        fields: indexConfig,
      });
    } else if (indexConfig.staged) {
      // indexConfig is object with fields and staged: true
      this.stagedDbIndexes.push({
        indexDescriptor: name,
        fields: indexConfig.fields,
      });
    } else {
      // indexConfig is object with fields (and maybe staged: false/undefined)
      this.indexes.push({
        indexDescriptor: name,
        fields: indexConfig.fields,
      });
    }
    return this;
  }

  /**
   * Define a search index on this table.
   *
   * To learn about search indexes, see [Search](https://docs.convex.dev/text-search).
   *
   * @param name - The name of the index.
   * @param indexConfig - The search index configuration object.
   * @returns A {@link TableDefinition} with this search index included.
   */
  searchIndex<
    IndexName extends string,
    SearchField extends ExtractFieldPaths<DocumentType>,
    FilterFields extends ExtractFieldPaths<DocumentType> = never,
  >(
    name: IndexName,
    indexConfig: Expand<
      SearchIndexConfig<SearchField, FilterFields> &
        IndexOptions & { staged?: false }
    >,
  ): TableDefinition<
    DocumentType,
    Indexes,
    // Update `SearchIndexes` to include the new index and use `Expand` to make
    // the types look pretty in editors.
    Expand<
      SearchIndexes &
        Record<
          IndexName,
          {
            searchField: SearchField;
            filterFields: FilterFields;
          }
        >
    >,
    VectorIndexes,
    FlowFields,
    ComputedFields,
    FlowFilters,
    FlowFieldRefs
  >;

  /**
   * Define a staged search index on this table.
   *
   * For large tables, index backfill can be slow. Staging an index allows you
   * to push the schema and enable the index later.
   *
   * If `staged` is `true`, the index will be staged and will not be enabled
   * until the staged flag is removed. Staged indexes do not block push
   * completion. Staged indexes cannot be used in queries.
   *
   * To learn about search indexes, see [Search](https://docs.convex.dev/text-search).
   *
   * @param name - The name of the index.
   * @param indexConfig - The search index configuration object.
   * @returns A {@link TableDefinition} with this search index included.
   */
  searchIndex<
    IndexName extends string,
    SearchField extends ExtractFieldPaths<DocumentType>,
    FilterFields extends ExtractFieldPaths<DocumentType> = never,
  >(
    name: IndexName,
    indexConfig: Expand<
      SearchIndexConfig<SearchField, FilterFields> &
        IndexOptions & { staged: true }
    >,
  ): TableDefinition<DocumentType, Indexes, SearchIndexes, VectorIndexes, FlowFields, ComputedFields, FlowFilters, FlowFieldRefs>;

  searchIndex<
    IndexName extends string,
    SearchField extends ExtractFieldPaths<DocumentType>,
    FilterFields extends ExtractFieldPaths<DocumentType> = never,
  >(
    name: IndexName,
    indexConfig: Expand<
      SearchIndexConfig<SearchField, FilterFields> & IndexOptions
    >,
  ) {
    if (indexConfig.staged) {
      this.stagedSearchIndexes.push({
        indexDescriptor: name,
        searchField: indexConfig.searchField,
        filterFields: indexConfig.filterFields || [],
      });
    } else {
      this.searchIndexes.push({
        indexDescriptor: name,
        searchField: indexConfig.searchField,
        filterFields: indexConfig.filterFields || [],
      });
    }
    return this;
  }

  /**
   * Define a vector index on this table.
   *
   * To learn about vector indexes, see [Vector Search](https://docs.convex.dev/vector-search).
   *
   * @param name - The name of the index.
   * @param indexConfig - The vector index configuration object.
   * @returns A {@link TableDefinition} with this vector index included.
   */
  vectorIndex<
    IndexName extends string,
    VectorField extends ExtractFieldPaths<DocumentType>,
    FilterFields extends ExtractFieldPaths<DocumentType> = never,
  >(
    name: IndexName,
    indexConfig: Expand<
      VectorIndexConfig<VectorField, FilterFields> &
        IndexOptions & { staged?: false }
    >,
  ): TableDefinition<
    DocumentType,
    Indexes,
    SearchIndexes,
    Expand<
      VectorIndexes &
        Record<
          IndexName,
          {
            vectorField: VectorField;
            dimensions: number;
            filterFields: FilterFields;
          }
        >
    >,
    FlowFields,
    ComputedFields,
    FlowFilters,
    FlowFieldRefs
  >;

  /**
   * Define a staged vector index on this table.
   *
   * For large tables, index backfill can be slow. Staging an index allows you
   * to push the schema and enable the index later.
   *
   * If `staged` is `true`, the index will be staged and will not be enabled
   * until the staged flag is removed. Staged indexes do not block push
   * completion. Staged indexes cannot be used in queries.
   *
   * To learn about vector indexes, see [Vector Search](https://docs.convex.dev/vector-search).
   *
   * @param name - The name of the index.
   * @param indexConfig - The vector index configuration object.
   * @returns A {@link TableDefinition} with this vector index included.
   */
  vectorIndex<
    IndexName extends string,
    VectorField extends ExtractFieldPaths<DocumentType>,
    FilterFields extends ExtractFieldPaths<DocumentType> = never,
  >(
    name: IndexName,
    indexConfig: Expand<
      VectorIndexConfig<VectorField, FilterFields> &
        IndexOptions & { staged: true }
    >,
  ): TableDefinition<DocumentType, Indexes, SearchIndexes, VectorIndexes, FlowFields, ComputedFields, FlowFilters, FlowFieldRefs>;

  vectorIndex<
    IndexName extends string,
    VectorField extends ExtractFieldPaths<DocumentType>,
    FilterFields extends ExtractFieldPaths<DocumentType> = never,
  >(
    name: IndexName,
    indexConfig: Expand<
      VectorIndexConfig<VectorField, FilterFields> & IndexOptions
    >,
  ) {
    if (indexConfig.staged) {
      this.stagedVectorIndexes.push({
        indexDescriptor: name,
        vectorField: indexConfig.vectorField,
        dimensions: indexConfig.dimensions,
        filterFields: indexConfig.filterFields || [],
      });
    } else {
      this.vectorIndexes.push({
        indexDescriptor: name,
        vectorField: indexConfig.vectorField,
        dimensions: indexConfig.dimensions,
        filterFields: indexConfig.filterFields || [],
      });
    }
    return this;
  }

  /**
   * Define a FlowField on this table.
   *
   * FlowFields are cross-table aggregations (sum, count, avg, min, max)
   * resolved via SQL at read time. They are read-only and not stored.
   *
   * @param name - The name of the FlowField.
   * @param config - The FlowField configuration.
   * @returns A {@link TableDefinition} with this FlowField included.
   */
  flowField<
    Name extends string,
    Returns extends Validator<any, any, any>,
    Source extends string = string,
    Key extends string = string,
    Field extends string = never,
  >(
    name: Name,
    config: FlowFieldConfig<
      Returns,
      StringKeysOf<FlowFilters>,
      Source,
      Key,
      Field
    >,
  ): TableDefinition<
    DocumentType,
    Indexes,
    SearchIndexes,
    VectorIndexes,
    Expand<FlowFields & Record<Name, Returns["type"]>>,
    ComputedFields,
    FlowFilters,
    Expand<
      FlowFieldRefs &
        Record<Name, { source: Source; key: Key; field: Field }>
    >
  >;
  flowField(name: string, config: FlowFieldConfig) {
    this.flowFieldDefs.push({
      fieldName: name,
      returns: config.returns.json,
      aggregation: config.type,
      source: config.source,
      key: config.key,
      field: config.field,
      filter: config.filter,
    });
    return this;
  }

  /**
   * Define a ComputedField on this table.
   *
   * ComputedFields are row-level expressions evaluated from stored fields
   * and FlowField values. They are read-only and not stored.
   *
   * @param name - The name of the ComputedField.
   * @param config - The ComputedField configuration.
   * @returns A {@link TableDefinition} with this ComputedField included.
   */
  computed<
    Name extends string,
    Returns extends Validator<any, any, any>,
  >(
    name: Name,
    config: ComputedFieldConfig<
      Returns,
      | StringKeysOf<ExtractDocument<DocumentType>>
      | "_id"
      | StringKeysOf<FlowFields>
      | StringKeysOf<ComputedFields>
    >,
  ): TableDefinition<
    DocumentType,
    Indexes,
    SearchIndexes,
    VectorIndexes,
    FlowFields,
    Expand<ComputedFields & Record<Name, Returns["type"]>>,
    FlowFilters,
    FlowFieldRefs
  >;
  computed(name: string, config: ComputedFieldConfig) {
    this.computedFieldDefs.push({
      fieldName: name,
      returns: config.returns.json,
      expr: config.expr,
    });
    return this;
  }

  /**
   * Define a FlowFilter on this table.
   *
   * FlowFilters are runtime parameters that parameterize FlowField
   * aggregations. They are not stored and do not appear in documents.
   *
   * @param name - The name of the FlowFilter.
   * @param config - The FlowFilter configuration.
   * @returns A {@link TableDefinition} with this FlowFilter included.
   */
  flowFilter<
    Name extends string,
    FilterType extends Validator<any, any, any>,
  >(
    name: Name,
    config: FlowFilterConfig<FilterType>,
  ): TableDefinition<
    DocumentType,
    Indexes,
    SearchIndexes,
    VectorIndexes,
    FlowFields,
    ComputedFields,
    Expand<FlowFilters & Record<Name, FilterType["type"]>>,
    FlowFieldRefs
  >;
  flowFilter(name: string, config: FlowFilterConfig) {
    this.flowFilterDefs.push({
      fieldName: name,
      filterType: config.type.json,
    });
    return this;
  }

  /**
   * Work around for https://github.com/microsoft/TypeScript/issues/57035
   */
  protected self(): TableDefinition<
    DocumentType,
    Indexes,
    SearchIndexes,
    VectorIndexes,
    FlowFields,
    ComputedFields,
    FlowFilters,
    FlowFieldRefs
  > {
    return this;
  }
  /**
   * Export the contents of this definition.
   *
   * This is called internally by the Convex framework.
   * @internal
   */
  export() {
    const documentType = this.validator.json;
    if (typeof documentType !== "object") {
      throw new Error(
        "Invalid validator: please make sure that the parameter of `defineTable` is valid (see https://docs.convex.dev/database/schemas)",
      );
    }

    return {
      indexes: this.indexes,
      stagedDbIndexes: this.stagedDbIndexes,
      searchIndexes: this.searchIndexes,
      stagedSearchIndexes: this.stagedSearchIndexes,
      vectorIndexes: this.vectorIndexes,
      stagedVectorIndexes: this.stagedVectorIndexes,
      documentType,
      flowFields: this.flowFieldDefs,
      computedFields: this.computedFieldDefs,
      flowFilters: this.flowFilterDefs,
    };
  }
}

/**
 * Define a table in a schema.
 *
 * You can either specify the schema of your documents as an object like
 * ```ts
 * defineTable({
 *   field: v.string()
 * });
 * ```
 *
 * or as a schema type like
 * ```ts
 * defineTable(
 *  v.union(
 *    v.object({...}),
 *    v.object({...})
 *  )
 * );
 * ```
 *
 * @param documentSchema - The type of documents stored in this table.
 * @returns A {@link TableDefinition} for the table.
 *
 * @public
 */
export function defineTable<
  DocumentSchema extends Validator<Record<string, any>, "required", any>,
>(documentSchema: DocumentSchema): TableDefinition<DocumentSchema>;
/**
 * Define a table in a schema.
 *
 * You can either specify the schema of your documents as an object like
 * ```ts
 * defineTable({
 *   field: v.string()
 * });
 * ```
 *
 * or as a schema type like
 * ```ts
 * defineTable(
 *  v.union(
 *    v.object({...}),
 *    v.object({...})
 *  )
 * );
 * ```
 *
 * @param documentSchema - The type of documents stored in this table.
 * @returns A {@link TableDefinition} for the table.
 *
 * @public
 */
export function defineTable<
  DocumentSchema extends Record<string, GenericValidator>,
>(
  documentSchema: DocumentSchema,
): TableDefinition<VObject<ObjectType<DocumentSchema>, DocumentSchema>>;
export function defineTable<
  DocumentSchema extends
    | Validator<Record<string, any>, "required", any>
    | Record<string, GenericValidator>,
>(documentSchema: DocumentSchema): TableDefinition<any, any, any> {
  if (isValidator(documentSchema)) {
    return new TableDefinition(documentSchema);
  } else {
    return new TableDefinition(v.object(documentSchema));
  }
}

/**
 * A type describing the schema of a Convex project.
 *
 * This should be constructed using {@link defineSchema}, {@link defineTable},
 * and {@link v}.
 * @public
 */
export type GenericSchema = Record<string, TableDefinition>;

/**
 *
 * The definition of a Convex project schema.
 *
 * This should be produced by using {@link defineSchema}.
 * @public
 */
/**
 * Extract field paths from a table in the schema (stored + system fields).
 */
type FieldPathsOfTable<
  Schema extends GenericSchema,
  TableName extends keyof Schema & string,
> = Schema[TableName] extends TableDefinition<
  infer DocType,
  any,
  any,
  any,
  any,
  any,
  any,
  any
>
  ? ExtractFieldPaths<DocType> | "_id" | "_creationTime"
  : string;

/**
 * Extract flow filter names from a table in the schema.
 */
type FlowFilterNamesOfTable<
  Schema extends GenericSchema,
  TableName extends keyof Schema & string,
> = Schema[TableName] extends TableDefinition<
  any,
  any,
  any,
  any,
  any,
  any,
  infer FFL,
  any
>
  ? StringKeysOf<FFL>
  : string;

/**
 * Extract all available fields for computed expressions from a table.
 */
type ComputedExprFieldsOfTable<
  Schema extends GenericSchema,
  TableName extends keyof Schema & string,
> = Schema[TableName] extends TableDefinition<
  infer DocType,
  any,
  any,
  any,
  infer FF,
  infer CF,
  any,
  any
>
  ?
      | StringKeysOf<ExtractDocument<DocType>>
      | "_id"
      | StringKeysOf<FF>
      | StringKeysOf<CF>
  : string;

/**
 * Update a table's FlowFields in the schema type.
 */
type SchemaWithFlowField<
  Schema extends GenericSchema,
  TableName extends keyof Schema & string,
  Name extends string,
  ReturnType,
  Source extends string,
  Key extends string,
  Field extends string,
> = {
  [K in keyof Schema]: K extends TableName
    ? Schema[K] extends TableDefinition<
        infer D,
        infer I,
        infer SI,
        infer VI,
        infer FF,
        infer CF,
        infer FFL,
        infer Refs
      >
      ? TableDefinition<
          D,
          I,
          SI,
          VI,
          Expand<FF & Record<Name, ReturnType>>,
          CF,
          FFL,
          Expand<
            Refs &
              Record<Name, { source: Source; key: Key; field: Field }>
          >
        >
      : Schema[K]
    : Schema[K];
};

/**
 * Update a table's ComputedFields in the schema type.
 */
type SchemaWithComputedField<
  Schema extends GenericSchema,
  TableName extends keyof Schema & string,
  Name extends string,
  ReturnType,
> = {
  [K in keyof Schema]: K extends TableName
    ? Schema[K] extends TableDefinition<
        infer D,
        infer I,
        infer SI,
        infer VI,
        infer FF,
        infer CF,
        infer FFL,
        infer Refs
      >
      ? TableDefinition<
          D,
          I,
          SI,
          VI,
          FF,
          Expand<CF & Record<Name, ReturnType>>,
          FFL,
          Refs
        >
      : Schema[K]
    : Schema[K];
};

/**
 * Update a table's FlowFilters in the schema type.
 */
type SchemaWithFlowFilter<
  Schema extends GenericSchema,
  TableName extends keyof Schema & string,
  Name extends string,
  FilterType,
> = {
  [K in keyof Schema]: K extends TableName
    ? Schema[K] extends TableDefinition<
        infer D,
        infer I,
        infer SI,
        infer VI,
        infer FF,
        infer CF,
        infer FFL,
        infer Refs
      >
      ? TableDefinition<
          D,
          I,
          SI,
          VI,
          FF,
          CF,
          Expand<FFL & Record<Name, FilterType>>,
          Refs
        >
      : Schema[K]
    : Schema[K];
};

export class SchemaDefinition<
  Schema extends GenericSchema,
  StrictTableTypes extends boolean,
> {
  public tables: Schema;
  public strictTableNameTypes!: StrictTableTypes;
  public readonly schemaValidation: boolean;
  private additionalFlowFields: Record<string, SerializedFlowField[]>;
  private additionalComputedFields: Record<string, SerializedComputedField[]>;
  private additionalFlowFilters: Record<string, SerializedFlowFilter[]>;

  /**
   * @internal
   */
  constructor(tables: Schema, options?: DefineSchemaOptions<StrictTableTypes>) {
    this.tables = tables;
    this.schemaValidation =
      options?.schemaValidation === undefined ? true : options.schemaValidation;
    this.additionalFlowFields = {};
    this.additionalComputedFields = {};
    this.additionalFlowFilters = {};
  }

  /**
   * Define a FlowField on a table in this schema.
   *
   * This schema-level method provides full autocomplete for `source` (table names),
   * `key` and `field` (field paths of the source table), and `filter.$field`
   * (FlowFilter names).
   *
   * @param tableName - The table to add the FlowField to.
   * @param name - The name of the FlowField.
   * @param config - The FlowField configuration.
   * @returns This {@link SchemaDefinition} with the FlowField included.
   * @public
   */
  flowField<
    TableName extends keyof Schema & string,
    Name extends string,
    Returns extends Validator<any, any, any>,
    Source extends keyof Schema & string,
    Key extends FieldPathsOfTable<Schema, Source>,
    Field extends FieldPathsOfTable<Schema, Source> = never,
  >(
    tableName: TableName,
    name: Name,
    config: {
      returns: Returns;
      type: FlowFieldAggregationType;
      source: Source;
      key: Key;
      field?: Field;
      filter?: FlowFieldFilter<
        FlowFilterNamesOfTable<Schema, TableName>
      >;
    },
  ): SchemaDefinition<
    SchemaWithFlowField<
      Schema,
      TableName,
      Name,
      Returns["type"],
      Source,
      Key,
      Field
    >,
    StrictTableTypes
  > {
    const defs = this.additionalFlowFields[tableName] ?? [];
    defs.push({
      fieldName: name,
      returns: config.returns.json,
      aggregation: config.type,
      source: config.source,
      key: config.key,
      field: config.field,
      filter: config.filter,
    });
    this.additionalFlowFields[tableName] = defs;
    return this as any;
  }

  /**
   * Define a ComputedField on a table in this schema.
   *
   * This schema-level method provides autocomplete for `$fieldName`
   * references including FlowFields added via prior `.flowField()` calls.
   *
   * @param tableName - The table to add the ComputedField to.
   * @param name - The name of the ComputedField.
   * @param config - The ComputedField configuration.
   * @returns This {@link SchemaDefinition} with the ComputedField included.
   * @public
   */
  computed<
    TableName extends keyof Schema & string,
    Name extends string,
    Returns extends Validator<any, any, any>,
  >(
    tableName: TableName,
    name: Name,
    config: ComputedFieldConfig<
      Returns,
      ComputedExprFieldsOfTable<Schema, TableName>
    >,
  ): SchemaDefinition<
    SchemaWithComputedField<
      Schema,
      TableName,
      Name,
      Returns["type"]
    >,
    StrictTableTypes
  > {
    const defs = this.additionalComputedFields[tableName] ?? [];
    defs.push({
      fieldName: name,
      returns: config.returns.json,
      expr: config.expr,
    });
    this.additionalComputedFields[tableName] = defs;
    return this as any;
  }

  /**
   * Define a FlowFilter on a table in this schema.
   *
   * @param tableName - The table to add the FlowFilter to.
   * @param name - The name of the FlowFilter.
   * @param config - The FlowFilter configuration.
   * @returns This {@link SchemaDefinition} with the FlowFilter included.
   * @public
   */
  flowFilter<
    TableName extends keyof Schema & string,
    Name extends string,
    FilterType extends Validator<any, any, any>,
  >(
    tableName: TableName,
    name: Name,
    config: FlowFilterConfig<FilterType>,
  ): SchemaDefinition<
    SchemaWithFlowFilter<
      Schema,
      TableName,
      Name,
      FilterType["type"]
    >,
    StrictTableTypes
  > {
    const defs = this.additionalFlowFilters[tableName] ?? [];
    defs.push({
      fieldName: name,
      filterType: config.type.json,
    });
    this.additionalFlowFilters[tableName] = defs;
    return this as any;
  }

  /**
   * Export the contents of this definition.
   *
   * This is called internally by the Convex framework.
   * @internal
   */
  export(): string {
    return JSON.stringify({
      tables: Object.entries(this.tables).map(([tableName, definition]) => {
        const {
          indexes,
          stagedDbIndexes,
          searchIndexes,
          stagedSearchIndexes,
          vectorIndexes,
          stagedVectorIndexes,
          documentType,
          flowFields,
          computedFields,
          flowFilters,
        } = definition.export();
        const extraFF = this.additionalFlowFields[tableName] ?? [];
        const extraCF = this.additionalComputedFields[tableName] ?? [];
        const extraFFL = this.additionalFlowFilters[tableName] ?? [];
        return {
          tableName,
          indexes,
          stagedDbIndexes,
          searchIndexes,
          stagedSearchIndexes,
          vectorIndexes,
          stagedVectorIndexes,
          documentType,
          flowFields: [...flowFields, ...extraFF],
          computedFields: [...computedFields, ...extraCF],
          flowFilters: [...flowFilters, ...extraFFL],
        };
      }),
      schemaValidation: this.schemaValidation,
    });
  }
}

/**
 * Options for {@link defineSchema}.
 *
 * @public
 */
export interface DefineSchemaOptions<StrictTableNameTypes extends boolean> {
  /**
   * Whether Convex should validate at runtime that all documents match
   * your schema.
   *
   * If `schemaValidation` is `true`, Convex will:
   * 1. Check that all existing documents match your schema when your schema
   * is pushed.
   * 2. Check that all insertions and updates match your schema during mutations.
   *
   * If `schemaValidation` is `false`, Convex will not validate that new or
   * existing documents match your schema. You'll still get schema-specific
   * TypeScript types, but there will be no validation at runtime that your
   * documents match those types.
   *
   * By default, `schemaValidation` is `true`.
   */
  schemaValidation?: boolean;

  /**
   * Whether the TypeScript types should allow accessing tables not in the schema.
   *
   * If `strictTableNameTypes` is `true`, using tables not listed in the schema
   * will generate a TypeScript compilation error.
   *
   * If `strictTableNameTypes` is `false`, you'll be able to access tables not
   * listed in the schema and their document type will be `any`.
   *
   * `strictTableNameTypes: false` is useful for rapid prototyping.
   *
   * Regardless of the value of `strictTableNameTypes`, your schema will only
   * validate documents in the tables listed in the schema. You can still create
   * and modify other tables on the dashboard or in JavaScript mutations.
   *
   * By default, `strictTableNameTypes` is `true`.
   */
  strictTableNameTypes?: StrictTableNameTypes;
}

/**
 * Define the schema of this Convex project.
 *
 * This should be exported as the default export from a `schema.ts` file in
 * your `convex/` directory. The schema enables runtime validation of documents
 * and provides end-to-end TypeScript type safety.
 *
 * Every document in Convex automatically has two system fields:
 * - `_id` - a unique document ID with validator `v.id("tableName")`
 * - `_creationTime` - a creation timestamp with validator `v.number()`
 *
 * You do not need to include these in your schema definition, they are added
 * automatically.
 *
 * @example
 * ```ts
 * // convex/schema.ts
 * import { defineSchema, defineTable } from "convex/server";
 * import { v } from "convex/values";
 *
 * export default defineSchema({
 *   users: defineTable({
 *     name: v.string(),
 *     email: v.string(),
 *   }).index("by_email", ["email"]),
 *
 *   messages: defineTable({
 *     body: v.string(),
 *     userId: v.id("users"),
 *     channelId: v.id("channels"),
 *   }).index("by_channel", ["channelId"]),
 *
 *   channels: defineTable({
 *     name: v.string(),
 *   }),
 *
 *   // Discriminated union table:
 *   results: defineTable(
 *     v.union(
 *       v.object({ kind: v.literal("error"), message: v.string() }),
 *       v.object({ kind: v.literal("success"), value: v.number() }),
 *     )
 *   ),
 * });
 * ```
 *
 * **Best practice:** Always include all index fields in the index name. For
 * example, an index on `["field1", "field2"]` should be named
 * `"by_field1_field2"`.
 *
 * @param schema - A map from table name to {@link TableDefinition} for all of
 * the tables in this project.
 * @param options - Optional configuration. See {@link DefineSchemaOptions} for
 * a full description.
 * @returns The schema.
 *
 * @see https://docs.convex.dev/database/schemas
 * @public
 */
/**
 * Validates that all FlowField `source`/`key`/`field` references point to
 * existing tables and fields.  Resolves to `never` when valid, or a union
 * of human-readable error strings when invalid.
 */
type FlowFieldRefErrors<Schema extends GenericSchema> = {
  [Table in keyof Schema & string]: Schema[Table] extends TableDefinition<
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    infer Refs
  >
    ? {
        [FF in keyof Refs & string]: Refs[FF] extends {
          source: infer S extends string;
          key: infer K extends string;
          field: infer F;
        }
          ? S extends keyof Schema & string
            ? Schema[S] extends TableDefinition<
                infer SourceDoc,
                any,
                any,
                any,
                any,
                any,
                any,
                any
              >
              ? K extends
                  | ExtractFieldPaths<SourceDoc>
                  | keyof SystemFields
                  | "_id"
                ? [F] extends [never]
                  ? never // field omitted — valid
                  : F extends
                        | ExtractFieldPaths<SourceDoc>
                        | keyof SystemFields
                        | "_id"
                    ? never // field valid
                    : `FlowField "${FF}" on "${Table}": field "${F & string}" not found on source table "${S}"`
                : `FlowField "${FF}" on "${Table}": key "${K}" not found on source table "${S}"`
              : never
            : `FlowField "${FF}" on "${Table}": source table "${S}" does not exist in the schema`
          : never;
      }[keyof Refs & string]
    : never;
}[keyof Schema & string];

export function defineSchema<
  Schema extends GenericSchema,
  StrictTableNameTypes extends boolean = true,
>(
  schema: FlowFieldRefErrors<Schema> extends never
    ? Schema
    : Schema & FlowFieldRefErrors<Schema>,
  options?: DefineSchemaOptions<StrictTableNameTypes>,
): SchemaDefinition<Schema, StrictTableNameTypes> {
  return new SchemaDefinition(schema, options);
}

/**
 * Internal type used in Convex code generation!
 *
 * Convert a {@link SchemaDefinition} into a {@link server.GenericDataModel}.
 *
 * @public
 */
export type DataModelFromSchemaDefinition<
  SchemaDef extends SchemaDefinition<any, boolean>,
> = MaybeMakeLooseDataModel<
  {
    [TableName in keyof SchemaDef["tables"] &
      string]: SchemaDef["tables"][TableName] extends TableDefinition<
      infer DocumentType,
      infer Indexes,
      infer SearchIndexes,
      infer VectorIndexes,
      infer FlowFields,
      infer ComputedFields,
      infer _FlowFilters,
      infer _FlowFieldRefs
    >
      ? {
          // We've already added all of the system fields except for `_id`.
          // Add that here. FlowFields and ComputedFields are merged into
          // the read-side document type.
          document: Expand<
            IdField<TableName> &
              ExtractDocument<DocumentType> &
              FlowFields &
              ComputedFields
          >;
          fieldPaths:
            | keyof IdField<TableName>
            | ExtractFieldPaths<DocumentType>;
          indexes: Expand<Indexes & SystemIndexes>;
          searchIndexes: SearchIndexes;
          vectorIndexes: VectorIndexes;
          // Track read-only field names so write operations can exclude them.
          computedFields: keyof FlowFields | keyof ComputedFields;
        }
      : never;
  },
  SchemaDef["strictTableNameTypes"]
>;

type MaybeMakeLooseDataModel<
  DataModel extends GenericDataModel,
  StrictTableNameTypes extends boolean,
> = StrictTableNameTypes extends true
  ? DataModel
  : Expand<DataModel & AnyDataModel>;

const _systemSchema = defineSchema({
  _scheduled_functions: defineTable({
    name: v.string(),
    args: v.array(v.any()),
    scheduledTime: v.float64(),
    completedTime: v.optional(v.float64()),
    state: v.union(
      v.object({ kind: v.literal("pending") }),
      v.object({ kind: v.literal("inProgress") }),
      v.object({ kind: v.literal("success") }),
      v.object({ kind: v.literal("failed"), error: v.string() }),
      v.object({ kind: v.literal("canceled") }),
    ),
  }),
  _storage: defineTable({
    sha256: v.string(),
    size: v.float64(),
    contentType: v.optional(v.string()),
  }),
});

export interface SystemDataModel
  extends DataModelFromSchemaDefinition<typeof _systemSchema> {}

export type SystemTableNames = TableNamesInDataModel<SystemDataModel>;
