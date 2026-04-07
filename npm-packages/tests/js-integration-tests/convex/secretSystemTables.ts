import {
  GenericDataModel,
  GenericDatabaseReader,
  GenericDatabaseWriter,
} from "convex/server";
/**
 * System tables can only be queried with `db.system`, but `db.system` uses the public types of virtual tables,
 * the way we intend to expose system tables to developers.
 * In order to use system tables types which are not exposed in `convex/server` in some of our tests,
 * use normal data model types with the db.system at runtime.
 */

// These "masks" are to let us test runtime behavior while still checking type errors.
export function maskSystem<T extends GenericDataModel>(
  db: GenericDatabaseReader<T>,
): GenericDatabaseReader<T> {
  return db.system as any as typeof db;
}

export function maskSystemWriter<T extends GenericDataModel>(
  db: GenericDatabaseWriter<T>,
): GenericDatabaseWriter<T> {
  return db.system as typeof db;
}
