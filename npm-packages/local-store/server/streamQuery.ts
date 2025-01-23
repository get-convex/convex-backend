import { streamQuery } from "../shared/pagination";

import {
  DataModelFromSchemaDefinition,
  GenericQueryCtx,
  SchemaDefinition,
  TableNamesInDataModel,
} from "convex/server";
import { PageRequest } from "../shared/pagination";

export const streamQueryForServerSchema = <
  ServerSchema extends SchemaDefinition<any, any>,
>(
  schema: ServerSchema,
) => {
  return <
    T extends TableNamesInDataModel<
      DataModelFromSchemaDefinition<ServerSchema>
    >,
  >(
    ctx: GenericQueryCtx<DataModelFromSchemaDefinition<ServerSchema>>,
    request: Omit<
      PageRequest<DataModelFromSchemaDefinition<ServerSchema>, T>,
      "targetMaxRows" | "absoluteMaxRows" | "schema"
    >,
  ) => {
    return streamQuery(ctx, { ...request, schema });
  };
};
