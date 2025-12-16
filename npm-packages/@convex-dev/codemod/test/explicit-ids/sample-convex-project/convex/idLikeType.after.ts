import { DatabaseReader } from "./_generated/server";

// This test ensures that the autofix works when the ID type isn’t a direct
// reference to the `Id` type, similarly to what the migrations component does.
async function _withIdLikeType(
  db: DatabaseReader,
  id: string & { __tableName: "documents" },
) {
  await db.get("documents", id);
}

async function _withWrongIdLikeType(
  db: DatabaseReader,
  id: string & { __tableName: any },
) {
  await db.get(
    id /* WARNING: Can’t update call site / Expected `id` to be an `Id<T>`, where `T` is a string literal, but got `T = any` instead. */,
  );
}
