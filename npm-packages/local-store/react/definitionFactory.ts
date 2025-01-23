import { SchemaDefinition } from "convex/server";
import { DefaultFunctionArgs, FunctionReference } from "convex/server";
import { LocalDbReader, LocalDbWriter } from "./localDb";
import { DataModelFromSchemaDefinition } from "convex/server";
import { Value } from "convex/values";

export interface LocalMutation<
  ServerArgs extends DefaultFunctionArgs,
  OptimisticUpdateArgs extends DefaultFunctionArgs = ServerArgs,
> {
  fn: FunctionReference<"mutation", "public", ServerArgs>;
  optimisticUpdate: (
    ctx: { localDb: LocalDbWriter<any> },
    args: OptimisticUpdateArgs,
  ) => void;
  serverArgs: (args: OptimisticUpdateArgs) => ServerArgs;
  __localMutation: true;
}

export interface LocalQuery<Args extends DefaultFunctionArgs, T extends Value> {
  handler: (ctx: { localDb: LocalDbReader<any> }, args: Args) => T;
  debugName?: string;
  __localQuery: true;
}

export class DefinitionFactory<SchemaDef extends SchemaDefinition<any, any>> {
  constructor(private syncSchema: SchemaDef) {}

  defineLocalMutation<
    ServerArgs extends DefaultFunctionArgs,
    OptimisticUpdateArgs extends DefaultFunctionArgs = ServerArgs,
  >(
    fn: FunctionReference<"mutation", "public", ServerArgs>,
    optimisticUpdate: (
      ctx: { localDb: LocalDbWriter<DataModelFromSchemaDefinition<SchemaDef>> },
      args: OptimisticUpdateArgs,
    ) => void,
    serverArgs?: (args: OptimisticUpdateArgs) => ServerArgs,
  ): LocalMutation<ServerArgs, OptimisticUpdateArgs> {
    return {
      fn,
      optimisticUpdate,
      serverArgs:
        serverArgs ??
        ((args: OptimisticUpdateArgs) => args as unknown as ServerArgs),
      __localMutation: true,
    };
  }

  defineLocalQuery<Args extends DefaultFunctionArgs, T extends Value>(
    f: (
      ctx: { localDb: LocalDbReader<DataModelFromSchemaDefinition<SchemaDef>> },
      args: Args,
    ) => T,
    debugName?: string,
  ): LocalQuery<Args, T> {
    return {
      handler: f,
      debugName,
      __localQuery: true,
    };
  }
}
