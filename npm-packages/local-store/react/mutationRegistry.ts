import {
  DataModelFromSchemaDefinition,
  FunctionReference,
  SchemaDefinition,
  getFunctionName,
} from "convex/server";
import { LocalDbWriter } from "./localDb";
import { LocalMutation } from "./definitionFactory";

export class MutationRegistry<SchemaDef extends SchemaDefinition<any, any>> {
  constructor(private _syncSchema: SchemaDef) {}

  private mutations: Record<
    string,
    {
      fn: FunctionReference<"mutation", "public">;
      optimisticUpdate: (
        ctx: {
          localDb: LocalDbWriter<DataModelFromSchemaDefinition<SchemaDef>>;
        },
        args: any,
      ) => void;
      serverArgs: (args: any) => any;
    }
  > = {};
  register(mutation: LocalMutation<any, any>) {
    const name = getFunctionName(mutation.fn);
    if (this.mutations[name]) {
      throw new Error(`Mutation ${name} already registered`);
    }
    this.mutations[name] = mutation;
    return this;
  }

  exportToMutationMap() {
    return this.mutations;
  }
}
