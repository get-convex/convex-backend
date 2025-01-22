import {
  DocumentByName,
  GenericDataModel,
  TableNamesInDataModel,
} from "convex/server";
import { GenericId, v } from "convex/values";

export type TriggerArgs<
  DataModel extends GenericDataModel,
  TableName extends TableNamesInDataModel<DataModel>,
> = {
  change: {
    type: "insert" | "patch" | "replace" | "delete";
    id: GenericId<TableName>;
    oldDoc: DocumentByName<DataModel, TableName> | null;
    newDoc: DocumentByName<DataModel, TableName> | null;
  };
};

export function triggerArgsValidator<
  DataModel extends GenericDataModel,
  TableName extends TableNamesInDataModel<DataModel>,
>(table: TableName) {
  return {
    change: v.object({
      type: v.union(
        v.literal("insert"),
        v.literal("patch"),
        v.literal("replace"),
        v.literal("delete"),
      ),
      id: v.id(table),
      oldDoc: v.any(),
      newDoc: v.any(),
    }),
  };
}
