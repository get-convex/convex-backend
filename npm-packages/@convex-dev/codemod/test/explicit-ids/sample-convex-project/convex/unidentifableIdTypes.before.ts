import { Id } from "./_generated/dataModel";
import { DatabaseReader } from "./_generated/server";

async function _anyId(db: DatabaseReader, id: any) {
  await db.get(
    id /* WARNING: Can’t update call site / Sorry, we can’t infer the table type of `id` (which is a `any`). */,
  );
}

async function _idAnyId(db: DatabaseReader, id: Id<any>) {
  await db.get(
    id /* WARNING: Can’t update call site / Expected `id` to be an `Id<T>`, where `T` is a string literal, but got `T = any` instead. */,
  );
}
