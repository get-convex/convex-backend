import {
  GenericDatabaseReader,
  GenericDatabaseWriter,
  GenericDataModel,
} from "convex/server";
import { Id } from "./_generated/dataModel";
import { DatabaseReader, DatabaseWriter } from "./_generated/server";

async function _fromDbReader(db: DatabaseReader, id: Id<"documents">) {
  await db.get(id);
}

async function _fromGenericDbReader(
  db: GenericDatabaseReader<GenericDataModel>,
  id: Id<"documents">,
) {
  await db.get(id);
}

async function _fromGenericDbReaderExtends<
  SomeDataModel extends GenericDataModel,
>(db: GenericDatabaseReader<SomeDataModel>, id: Id<"documents">) {
  await db.get(id);
}

async function _fromDbWriter(db: DatabaseWriter, id: Id<"documents">) {
  await db.get(id);
  await db.replace(id, {
    name: "test2",
  });
  await db.patch(id, {
    name: "test3",
  });
  await db.delete(id);
}

async function _fromGenericDbWriter(
  db: GenericDatabaseWriter<GenericDataModel>,
  id: Id<"documents">,
) {
  await db.get(id);
  await db.replace(id, {
    name: "test2",
  });
  await db.patch(id, {
    name: "test3",
  });
  await db.delete(id);
}
