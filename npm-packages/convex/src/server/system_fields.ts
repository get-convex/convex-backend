import { GenericId } from "../values/index.js";
import { BetterOmit, Expand } from "../type_utils.js";
import { GenericDocument, GenericTableInfo } from "./data_model.js";

/**
 * The fields that Convex automatically adds to documents, not including `_id`.
 *
 * This is an object type mapping field name to field type.
 * @public
 */
export type SystemFields = {
  _creationTime: number;
};

/**
 * The `_id` field that Convex automatically adds to documents.
 * @public
 */
export type IdField<TableName extends string> = {
  _id: GenericId<TableName>;
};

/**
 * A Convex document with the system fields like `_id` and `_creationTime` omitted.
 *
 * @public
 */
export type WithoutSystemFields<Document extends GenericDocument> = Expand<
  BetterOmit<Document, keyof SystemFields | "_id">
>;

/**
 * A Convex document with the system fields like `_id` and `_creationTime` optional.
 *
 * @public
 */
export type WithOptionalSystemFields<Document extends GenericDocument> = Expand<
  WithoutSystemFields<Document> &
    Partial<Pick<Document, keyof SystemFields | "_id">>
>;

/**
 * Helper: resolves to the computed field names when they are specific, or
 * `never` when the generic `string` type is used (so we don't accidentally
 * strip all keys in the untyped / GenericDataModel case).
 */
type ResolvedComputedFields<TableInfo extends GenericTableInfo> =
  string extends TableInfo["computedFields"]
    ? never
    : TableInfo["computedFields"];

/**
 * The writable fields for `insert`: excludes system fields, `_id`, and
 * read-only computed fields (FlowFields and ComputedFields).
 *
 * @public
 */
export type WritableFields<TableInfo extends GenericTableInfo> = Expand<
  BetterOmit<
    TableInfo["document"],
    keyof SystemFields | "_id" | ResolvedComputedFields<TableInfo>
  >
>;

/**
 * The value type for `replace`: excludes read-only computed fields and makes
 * system fields optional.
 *
 * @public
 */
export type ReplaceValue<TableInfo extends GenericTableInfo> = Expand<
  BetterOmit<
    TableInfo["document"],
    keyof SystemFields | "_id" | ResolvedComputedFields<TableInfo>
  > &
    Partial<Pick<TableInfo["document"], keyof SystemFields | "_id">>
>;

/**
 * The indexes that Convex automatically adds to every table.
 *
 * This is an object mapping index names to index field paths.
 * @public
 */
export type SystemIndexes = {
  // Note `db.get(id)` is simpler and equivalent to a query on `by_id`.
  // Unless the query is being built dynamically, or doing manual pagination.
  by_id: ["_id"];

  by_creation_time: ["_creationTime"];
};

/**
 * Convex automatically appends "_creationTime" to the end of every index to
 * break ties if all of the other fields are identical.
 * @public
 */
export type IndexTiebreakerField = "_creationTime";
