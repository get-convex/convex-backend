import {
  FieldTypeFromFieldPath,
  Indexes,
  NamedTableInfo,
  SystemTableNames,
} from "convex/server";
import { DataModel, Doc, TableNames } from "../_generated/dataModel";
import { DatabaseReader } from "../_generated/server";
import { GenericId } from "convex/values";

// We don't want to include system tables for joins as they have different
// patterns for .get and .query
type Id<TableName extends TableNames> = GenericId<TableName>;
/**
 * asyncMap returns the results of applying an async function over an list.
 *
 * @param list - Iterable object of items, e.g. an Array, Set, Object.keys
 * @param asyncTransform
 * @returns
 */
export async function asyncMap<FromType, ToType>(
  list: Iterable<FromType>,
  asyncTransform: (item: FromType) => Promise<ToType>,
): Promise<ToType[]> {
  const promises: Promise<ToType>[] = [];
  for (const item of list) {
    promises.push(asyncTransform(item));
  }
  return Promise.all(promises);
}

/**
 * getAll returns a list of Documents corresponding to the `Id`s passed in.
 * @param db A database object, usually passed from a mutation or query ctx.
 * @param ids An list (or other iterable) of Ids pointing to a table.
 * @returns The Documents referenced by the Ids, in order. `null` if not found.
 */
export async function getAll<TableName extends TableNames>(
  db: DatabaseReader,
  ids: Id<TableName>[],
): Promise<(Doc<TableName> | null)[]> {
  return asyncMap(ids, db.get);
}

// `FieldPath`s that have a `"FieldPath"` index on [`FieldPath`, ...]
// type LookupFieldPaths<TableName extends TableNames> =   {[FieldPath in DataModel[TableName]["fieldPaths"]]: FieldPath extends keyof DataModel[TableName]["indexes"]? Indexes<NamedTableInfo<DataModel, TableName>>[FieldPath][0] extends FieldPath ? FieldPath : never: never}[DataModel[TableName]["fieldPaths"]]

// `FieldPath`s that have a `"by_${FieldPath}""` index on [`FieldPath`, ...]
type LookupFieldPaths<TableName extends TableNames> = {
  [FieldPath in DataModel[TableName]["fieldPaths"]]: `by_${FieldPath}` extends keyof DataModel[TableName]["indexes"]
    ? Indexes<
        NamedTableInfo<DataModel, TableName>
      >[`by_${FieldPath}`][0] extends FieldPath
      ? FieldPath
      : never
    : never;
}[DataModel[TableName]["fieldPaths"]];

type TablesWithLookups = {
  [TableName in TableNames]: LookupFieldPaths<TableName> extends never
    ? never
    : TableName;
}[TableNames];

/**
 * Get a document that references a value with a field indexed `by_${field}`
 *
 * Useful for fetching a document with a one-to-one relationship via backref.
 * @param db DatabaseReader, passed in from the function ctx
 * @param table The table to fetch the target document from.
 * @param field The field on that table that should match the specified value.
 * @param value The value to look up the document by, usually an ID.
 * @returns The document matching the value, or null if none found.
 */
export async function getOneFrom<
  TableName extends TablesWithLookups,
  Field extends LookupFieldPaths<TableName>,
>(
  db: DatabaseReader,
  table: TableName,
  field: Field,
  value: FieldTypeFromFieldPath<Doc<TableName>, Field>,
): Promise<Doc<TableName> | null> {
  const ret = db
    .query(table)
    .withIndex("by_" + field, (q) => q.eq(field, value as any))
    .unique();
  return ret;
}

/**
 * Get a list of documents matching a value with a field indexed `by_${field}`.
 *
 * Useful for fetching many documents related to a given value via backrefs.
 * @param db DatabaseReader, passed in from the function ctx
 * @param table The table to fetch the target document from.
 * @param field The field on that table that should match the specified value.
 * @param value The value to look up the document by, usually an ID.
 * @returns The documents matching the value, if any.
 */
export async function getManyFrom<
  TableName extends TablesWithLookups,
  Field extends LookupFieldPaths<TableName>,
>(
  db: DatabaseReader,
  table: TableName,
  field: Field,
  value: FieldTypeFromFieldPath<Doc<TableName>, Field>,
): Promise<(Doc<TableName> | null)[]> {
  return db
    .query(table)
    .withIndex("by_" + field, (q) => q.eq(field, value as any))
    .collect();
}

// File paths to fields that are IDs, excluding "_id".
type IdFilePaths<
  InTableName extends TablesWithLookups,
  TableName extends TableNames,
> = {
  [FieldName in DataModel[InTableName]["fieldPaths"]]: FieldTypeFromFieldPath<
    Doc<InTableName>,
    FieldName
  > extends Id<TableName>
    ? TableName extends SystemTableNames
      ? never
      : FieldName extends "_id"
        ? never
        : FieldName
    : never;
}[DataModel[InTableName]["fieldPaths"]];

// Whether a table has an ID field that isn't its sole lookup field.
// These can operate as join tables, going from one table to another.
// One field has an indexed field for lookup, and another has the ID to get.
type LookupAndIdFilePaths<TableName extends TablesWithLookups> = {
  [FieldPath in IdFilePaths<
    TableName,
    TableNames
  >]: LookupFieldPaths<TableName> extends FieldPath ? never : true;
}[IdFilePaths<TableName, TableNames>];

// The table names that  match LookupAndIdFields.
// These are the possible "join" or "edge" or "relationship" tables.
type JoinTables = {
  [TableName in TablesWithLookups]: LookupAndIdFilePaths<TableName> extends never
    ? never
    : TableName;
}[TablesWithLookups];

// many-to-many via lookup table
/**
 * Get related documents by using a join table.
 *
 * It will find all join table entries matching a value, then look up all the
 * documents pointed to by the join table entries. Useful for many-to-many
 * relationships.
 * @param db DatabaseReader, passed in from the function ctx
 * @param table The table to fetch the target document from.
 * @param toField The ID field on the table pointing at target documents.
 * @param fromField The field on the table to compare to the value.
 * @param value The value to match the fromField on the table, usually an ID.
 * @returns The documents targeted by matching documents in the table, if any.
 */
export async function getManyVia<
  JoinTableName extends JoinTables,
  ToField extends IdFilePaths<JoinTableName, TableNames>,
  FromField extends Exclude<LookupFieldPaths<JoinTableName>, ToField>,
  TargetTableName extends TableNames = FieldTypeFromFieldPath<
    Doc<JoinTableName>,
    ToField
  > extends Id<infer TargetTableName>
    ? TargetTableName
    : never,
>(
  db: DatabaseReader,
  table: JoinTableName,
  toField: ToField,
  fromField: FromField,
  value: FieldTypeFromFieldPath<Doc<JoinTableName>, FromField>,
): Promise<(Doc<TargetTableName> | null)[]> {
  return asyncMap(await getManyFrom(db, table, fromField, value), (link) =>
    db.get((link as any)[toField]),
  );
}

/**
 * Filters out null elements from an array.
 * @param list List of elements that might be null.
 * @returns List of elements with nulls removed.
 */
export function pruneNull<T>(list: (T | null)[]): T[] {
  return list.filter((i) => i !== null) as T[];
}
