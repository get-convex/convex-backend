import {
  Expand,
  FunctionHandle,
  FunctionReference,
  GenericDataModel,
  GenericDatabaseWriter,
  GenericMutationCtx,
  QueryInitializer,
  TableNamesInDataModel,
  createFunctionHandle,
} from "convex/server";
import { api } from "../triggers/_generated/api.js";
import { GenericId } from "convex/values";
import { TriggerArgs } from "../types.js";
import { AtomicMutators, atomicMutators } from "./atomicMutators.js";

export type { AtomicMutators };
export { atomicMutators };
export { triggerArgsValidator } from "../types.js";

type InternalizeApi<API> = Expand<{
  [mod in keyof API]: API[mod] extends FunctionReference<any, any, any, any>
    ? FunctionReference<
        API[mod]["_type"],
        "internal",
        API[mod]["_args"],
        API[mod]["_returnType"],
        API[mod]["_componentPath"]
      >
    : InternalizeApi<API[mod]>;
}>;
type InstalledAPI = InternalizeApi<typeof api>;

export type Triggers<DataModel extends GenericDataModel> = {
  [TableName in TableNamesInDataModel<DataModel>]?: {
    atomicMutators: AtomicMutators;
    triggers: FunctionReference<
      "mutation",
      any,
      TriggerArgs<DataModel, TableName>,
      null
    >[];
  };
};

type TriggerHandles<DataModel extends GenericDataModel> = {
  [TableName in TableNamesInDataModel<DataModel>]?: {
    atomicMutators: {
      [Mutator in keyof AtomicMutators]: FunctionHandle<"mutation">;
    };
    triggers: FunctionHandle<
      "mutation",
      TriggerArgs<DataModel, TableName>,
      null
    >[];
  };
};

export type WithTriggers<DataModel extends GenericDataModel> = {
  args: Record<string, never>;
  input: (
    ctx: GenericMutationCtx<DataModel>,
    args: any,
  ) => Promise<{
    args: Record<string, never>;
    ctx: { db: WrapWriter<DataModel> };
  }>;
};

export function withTriggers<DataModel extends GenericDataModel>(
  api: InstalledAPI,
  triggers: Triggers<DataModel>,
): WithTriggers<DataModel> {
  return {
    args: {},
    input: async (ctx: GenericMutationCtx<DataModel>, _args: any) => {
      const handles: TriggerHandles<DataModel> = {};
      for (const tableNameStr of Object.keys(triggers)) {
        const tableName = tableNameStr as TableNamesInDataModel<DataModel>;
        const tableTrigger = triggers[tableName]!;
        handles[tableName] = {
          atomicMutators: {
            atomicInsert: await createFunctionHandle(
              tableTrigger.atomicMutators.atomicInsert,
            ),
            atomicPatch: await createFunctionHandle(
              tableTrigger.atomicMutators.atomicPatch,
            ),
            atomicReplace: await createFunctionHandle(
              tableTrigger.atomicMutators.atomicReplace,
            ),
            atomicDelete: await createFunctionHandle(
              tableTrigger.atomicMutators.atomicDelete,
            ),
          },
          triggers: await Promise.all(
            tableTrigger.triggers.map(createFunctionHandle),
          ),
        };
      }
      const db = new WrapWriter(ctx, api, handles);
      return {
        ctx: { db },
        args: {},
      };
    },
  };
}

class WrapWriter<DataModel extends GenericDataModel> {
  ctx: GenericMutationCtx<DataModel>;
  system: GenericDatabaseWriter<DataModel>["system"];
  api: InstalledAPI;
  triggers: TriggerHandles<DataModel>;

  constructor(
    ctx: GenericMutationCtx<DataModel>,
    api: InstalledAPI,
    triggers: TriggerHandles<DataModel>,
  ) {
    this.ctx = ctx;
    this.system = ctx.db.system;
    this.api = api;
    this.triggers = triggers;
  }
  normalizeId<TableName extends TableNamesInDataModel<DataModel>>(
    tableName: TableName,
    id: string,
  ): GenericId<TableName> | null {
    return this.ctx.db.normalizeId(tableName, id);
  }
  async insert<TableName extends string>(
    table: TableName,
    value: any,
  ): Promise<GenericId<TableName>> {
    if (table in this.triggers) {
      const tableTrigger = this.triggers[table]!;
      return await this.ctx.runMutation(this.api.documents.insert, {
        value,
        atomicInsert: tableTrigger.atomicMutators.atomicInsert,
        triggers: tableTrigger.triggers,
      });
    } else {
      return await this.ctx.db.insert(table, value);
    }
  }
  async patch<TableName extends string>(
    table: TableName,
    id: GenericId<TableName>,
    value: Partial<any>,
  ): Promise<void> {
    if (table in this.triggers) {
      const tableTrigger = this.triggers[table]!;
      await this.ctx.runMutation(this.api.documents.patch, {
        id,
        value,
        atomicPatch: tableTrigger.atomicMutators.atomicPatch,
        triggers: tableTrigger.triggers,
      });
    } else {
      await this.ctx.db.patch(id, value);
    }
  }
  async replace<TableName extends string>(
    table: TableName,
    id: GenericId<TableName>,
    value: any,
  ): Promise<void> {
    if (table in this.triggers) {
      const tableTrigger = this.triggers[table]!;
      await this.ctx.runMutation(this.api.documents.replace, {
        id,
        value,
        atomicReplace: tableTrigger.atomicMutators.atomicReplace,
        triggers: tableTrigger.triggers,
      });
    } else {
      await this.ctx.db.replace(id, value);
    }
  }
  async delete<TableName extends string>(
    table: TableName,
    id: GenericId<TableName>,
  ): Promise<void> {
    if (table in this.triggers) {
      const tableTrigger = this.triggers[table]!;
      await this.ctx.runMutation(this.api.documents.deleteDoc, {
        id,
        atomicDelete: tableTrigger.atomicMutators.atomicDelete,
        triggers: tableTrigger.triggers,
      });
    } else {
      await this.ctx.db.delete(id);
    }
  }
  get<TableName extends string>(id: GenericId<TableName>): Promise<any> {
    return this.ctx.db.get(id);
  }
  query<TableName extends string>(tableName: TableName): QueryInitializer<any> {
    return this.ctx.db.query(tableName);
  }
}
