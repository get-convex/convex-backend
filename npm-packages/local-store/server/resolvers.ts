// framework code imported from convex/server

import { GenericId, v } from "convex/values";
import {
  GeneratorCursor,
  IndexKey,
  isMaximal,
  isMinimal,
  Key,
  MAXIMAL_KEY,
  MINIMAL_KEY,
  PageArguments,
  PageResult,
} from "../shared/types";
import {
  AnyDataModel,
  DataModelFromSchemaDefinition,
  DocumentByName,
  GenericDocument,
  GenericQueryCtx,
  IndexNames,
  NamedTableInfo,
  queryGeneric,
  RegisteredQuery,
  SchemaDefinition,
  TableNamesInDataModel,
} from "convex/server";

export function indexFieldsForSyncObject(
  syncSchema: any,
  table: string,
  index: string,
) {
  const indexDefinition: any = syncSchema.tables[table].indexes.find(
    (i: any) => i.indexDescriptor === index,
  );
  if (!indexDefinition) {
    throw new Error(`Index ${index} not found for table ${table}`);
  }
  return indexDefinition.fields;
}

export function cursorForSyncObject(
  syncSchema: any,
  table: string,
  index: string,
  doc: any,
) {
  const fields = indexFieldsForSyncObject(syncSchema, table, index);
  // TODO: null is kind of wrong but we can't use undefined because it's not convex-json serializable
  return {
    kind: "exact" as const,
    value: fields.map((field: string) => doc[field] ?? null),
  };
}

async function getStartKey({
  ctx,
  indexName,
  indexResolver,
  tableResolver,
  getCursor,
  target,
}: {
  ctx: GenericQueryCtx<any>;
  indexName: string;
  indexResolver: IndexResolverGenerator<AnyDataModel>;
  tableResolver: TableResolver<any, any, any>;
  getCursor: (doc: GenericDocument) => GeneratorCursor;
  target: Key;
}): Promise<GeneratorCursor> {
  if (isMinimal(target)) {
    return MINIMAL_KEY;
  }
  if (isMaximal(target)) {
    return MAXIMAL_KEY;
  }
  if (target.kind === "exact") {
    return { kind: "exact", value: target.value };
  } else if (target.kind === "successor") {
    const stream = syncDocumentGenerator({
      ctx,
      tableResolver,
      generator: indexResolver,
      indexName,
      args: {
        key: target.value,
        inclusive: false,
        direction: "asc",
      },
    });
    const { value: firstResult, done: firstDone } = await stream().next();
    if (firstDone) {
      // if we are asking for the successor of something and we can't find anything after that something,
      // start from the end and walk backwards to find the last page.
      return MAXIMAL_KEY;
    }
    return getCursor(firstResult);
  } else if (target.kind === "predecessor") {
    const stream = syncDocumentGenerator({
      ctx,
      tableResolver,
      generator: indexResolver,
      indexName,
      args: {
        key: target.value,
        inclusive: false,
        direction: "desc",
      },
    });
    const { value: firstResult, done: firstDone } = await stream().next();
    if (firstDone) {
      return MINIMAL_KEY;
    }
    return getCursor(firstResult);
  }
  throw new Error(`Unexpected target kind ${(target as any).kind}`);
}

async function isPageBoundary(id: GenericId<any>, log2PageSize: number) {
  const mask = (1 << log2PageSize) - 1;

  const encoder = new TextEncoder();
  const data = encoder.encode(id);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  const randomInt = new DataView(hashBuffer).getUint32(0, true);

  return (randomInt & mask) === mask;
}

/**
 * This is a query we're going to optimistically update so we
 * can know when mutations have been reflected and their optimistic update has dropped.
 *
 * It's a total hack but it works.
 */
export const unreflectedMutations = queryGeneric((): Promise<string[]> => {
  return Promise.resolve([]);
});

type TableResolver<
  ServerSchema extends SchemaDefinition<any, any>,
  SyncSchema extends SchemaDefinition<any, any>,
  TableName extends TableNamesInDataModel<
    DataModelFromSchemaDefinition<SyncSchema>
  >,
> = {
  get: (
    ctx: GenericQueryCtx<DataModelFromSchemaDefinition<ServerSchema>>,
    _id: string,
  ) => Promise<DocumentByName<
    DataModelFromSchemaDefinition<SyncSchema>,
    TableName
  > | null>;
  tableName: TableName;
  syncSchema: SyncSchema;
};

type IndexResolver<
  ServerSchema extends SchemaDefinition<any, any>,
  SyncSchema extends SchemaDefinition<any, any>,
  TableName extends TableNamesInDataModel<
    DataModelFromSchemaDefinition<SyncSchema>
  >,
> = <
  IndexName extends IndexNames<
    NamedTableInfo<DataModelFromSchemaDefinition<SyncSchema>, TableName>
  >,
>(
  indexName: IndexName,
  generator: IndexResolverGenerator<
    DataModelFromSchemaDefinition<ServerSchema>
  >,
) => RegisteredQuery<"public", PageArguments, Promise<PageResult>>;

export const tableResolverFactory = <
  ServerSchema extends SchemaDefinition<any, any>,
  SyncSchema extends SchemaDefinition<any, any>,
>(
  syncSchema: SyncSchema,
  _serverSchema: ServerSchema,
) => {
  return {
    table: <
      Table extends TableNamesInDataModel<
        DataModelFromSchemaDefinition<SyncSchema>
      >,
    >(
      tableName: Table,
      get: (
        ctx: GenericQueryCtx<DataModelFromSchemaDefinition<ServerSchema>>,
        _id: string,
      ) => Promise<DocumentByName<
        DataModelFromSchemaDefinition<SyncSchema>,
        Table
      > | null>,
    ): {
      get: RegisteredQuery<
        "public",
        { _id: string },
        DocumentByName<DataModelFromSchemaDefinition<SyncSchema>, Table> | null
      >;
      index: IndexResolver<ServerSchema, SyncSchema, Table>;
    } => {
      const tableResolver = {
        get,
        tableName,
        syncSchema,
      };

      const indexResolver = resolverFactory(tableResolver);
      return {
        get: queryGeneric({
          args: { _id: v.string() },
          handler: (
            ctx: GenericQueryCtx<DataModelFromSchemaDefinition<ServerSchema>>,
            args: { _id: string },
          ) => get(ctx, args._id),
        }),
        index: indexResolver,
      };
    },
  };
};

export type IndexResolverGeneratorArgs = {
  key: IndexKey;
  inclusive: boolean;
  direction: "asc" | "desc";
};

export type IndexResolverGenerator<DM extends AnyDataModel = AnyDataModel> = (
  ctx: GenericQueryCtx<DM>,
  args: IndexResolverGeneratorArgs,
) => AsyncGenerator<string>;

export const resolverFactory = <
  ServerSchema extends SchemaDefinition<any, any>,
  SyncSchema extends SchemaDefinition<any, any>,
  Table extends TableNamesInDataModel<
    DataModelFromSchemaDefinition<SyncSchema>
  >,
>(
  tableResolver: TableResolver<ServerSchema, SyncSchema, Table>,
): IndexResolver<ServerSchema, SyncSchema, Table> => {
  return <
    Index extends IndexNames<
      NamedTableInfo<DataModelFromSchemaDefinition<SyncSchema>, Table>
    >,
  >(
    indexName: Index,
    generator: IndexResolverGenerator<
      DataModelFromSchemaDefinition<ServerSchema>
    >,
  ) => {
    return syncIndexResolverWithSchema(
      tableResolver,
      indexName as string,
      generator as unknown as any,
    );
  };
};

function syncDocumentGenerator({
  ctx,
  tableResolver,
  generator,
  indexName,
  args,
}: {
  ctx: GenericQueryCtx<any>;
  tableResolver: TableResolver<any, any, any>;
  generator: IndexResolverGenerator<AnyDataModel>;
  indexName: string;
  args: IndexResolverGeneratorArgs;
}) {
  return async function* () {
    for await (const resultId of generator(ctx, args)) {
      const result = await tableResolver.get(ctx, resultId);
      if (result === null) {
        console.warn(
          `[${tableResolver.tableName}.${indexName}] Filtering out document ${resultId} due to access control`,
        );
      } else {
        yield result;
      }
    }
  };
}

const syncIndexResolverWithSchema = (
  tableResolver: TableResolver<any, any, any>,
  indexName: string,
  generator: IndexResolverGenerator<AnyDataModel>,
): RegisteredQuery<"public", PageArguments, Promise<PageResult>> => {
  return queryGeneric({
    args: {
      syncTableName: v.literal(tableResolver.tableName),
      index: v.literal(indexName),
      target: v.any(),
      log2PageSize: v.number(),
    },
    handler: async (ctx, args): Promise<PageResult> => {
      const target = args.target as Key;
      const getCursor = (doc: any) =>
        cursorForSyncObject(
          tableResolver.syncSchema,
          args.syncTableName,
          args.index,
          doc,
        );

      const startKey = await getStartKey({
        ctx,
        tableResolver,
        getCursor,
        target,
        indexName,
        indexResolver: generator,
      });
      // now startKey is a key that we want to find a page containing.

      // console.log("startKey", startKey, "target", target);

      // First look backwards and include all results after the previous page boundary.
      const results = [];
      let lowerBound: GeneratorCursor = MINIMAL_KEY;
      if (!isMinimal(startKey)) {
        const streamBack = syncDocumentGenerator({
          ctx,
          tableResolver,
          generator,
          indexName,
          args: {
            key: startKey.value,
            inclusive: isMaximal(startKey),
            direction: "desc",
          },
        });

        for await (const result of streamBack()) {
          const isBoundary = await isPageBoundary(
            result._id,
            args.log2PageSize,
          );
          // console.log("result cursor", getCursor(result));
          // console.log("isBoundary", isBoundary);
          if (isBoundary) {
            lowerBound = {
              kind: "successor",
              value: getCursor(result).value,
            };
            break;
          }

          results.push(result);
        }
        // results is now documents in reverse cursor order excluding the target document
        // now reverse it
        results.reverse();
        // console.log("reversed results", results);
      }

      let upperBound: GeneratorCursor = MAXIMAL_KEY;
      if (!isMaximal(startKey)) {
        const stream = syncDocumentGenerator({
          ctx,
          tableResolver,
          generator,
          indexName,
          args: {
            key: startKey.value,
            inclusive: true,
            direction: "asc",
          },
        });
        for await (const result of stream()) {
          // Add the document even if it's a page boundary since we include the upper bound.
          results.push(result);
          if (await isPageBoundary(result._id, args.log2PageSize)) {
            upperBound = getCursor(result);
            break;
          }
        }
      }

      // console.log("sync query results", results, lowerBound, upperBound);
      return {
        results,
        lowerBound,
        upperBound,
      } as PageResult;
    },
  });
};
