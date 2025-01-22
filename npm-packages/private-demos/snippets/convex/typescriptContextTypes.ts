// Types based on your schema
import { Doc, Id } from "./_generated/dataModel";
// Types based on your schema and declared functions
import {
  QueryCtx,
  MutationCtx,
  ActionCtx,
  DatabaseReader,
  DatabaseWriter,
} from "./_generated/server";
// Types that don't depend on schema or function
import {
  Auth,
  StorageReader,
  StorageWriter,
  StorageActionWriter,
} from "convex/server";

// Note that a `MutationCtx` also satisfies the `QueryCtx` interface
export function myReadHelper(ctx: QueryCtx, id: Id<"channels">) {
  /* ... */
}

export function myActionHelper(ctx: ActionCtx, doc: Doc<"messages">) {
  /* ... */
}
