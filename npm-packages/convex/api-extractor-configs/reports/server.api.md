## API Report File for "convex"

> Do not edit this file. It is a report generated by [API Extractor](https://api-extractor.com/).

```ts

// @public
export type ActionBuilder<DataModel extends GenericDataModel, Visibility extends FunctionVisibility> = {
    <Output, ArgsValidator extends PropertyValidators>(func: ValidatedFunction<GenericActionCtx<DataModel>, ArgsValidator, Output>): RegisteredAction<Visibility, ObjectType<ArgsValidator>, Output>;
    <Output, Args extends ArgsArray = OneArgArray>(func: UnvalidatedFunction<GenericActionCtx<DataModel>, Args, Output>): RegisteredAction<Visibility, ArgsArrayToObject<Args>, Output>;
};

// @public @deprecated
export interface ActionCtx<DataModel extends GenericDataModel = GenericDataModel> extends GenericActionCtx<DataModel> {
}

// @public
export const actionGeneric: ActionBuilder<any, "public">;

// Warning: (ae-forgotten-export) The symbol "AnyModuleDirOrFunc" needs to be exported by the entry point index.d.ts
//
// @public
export type AnyApi = Record<string, Record<string, AnyModuleDirOrFunc>>;

// @public
export const anyApi: AnyApi;

// @public
export type AnyDataModel = {
    [tableName: string]: {
        document: any;
        fieldPaths: GenericFieldPaths;
        indexes: {};
        searchIndexes: {};
        vectorIndexes: {};
    };
};

// Warning: (ae-forgotten-export) The symbol "ApiFromModulesAllowEmptyNodes" needs to be exported by the entry point index.d.ts
//
// @public
export type ApiFromModules<AllModules extends Record<string, object>> = FilterApi<ApiFromModulesAllowEmptyNodes<AllModules>, FunctionReference<any, any, any, any>>;

// Warning: (ae-forgotten-export) The symbol "AnyFunctionReference" needs to be exported by the entry point index.d.ts
// Warning: (ae-forgotten-export) The symbol "EmptyObject" needs to be exported by the entry point index.d.ts
//
// @public
export type ArgsAndOptions<FuncRef extends AnyFunctionReference, Options> = FuncRef["_args"] extends EmptyObject ? [args?: EmptyObject, options?: Options] : [args: FuncRef["_args"], options?: Options];

// Warning: (ae-forgotten-export) The symbol "NoArgsArray" needs to be exported by the entry point index.d.ts
//
// @public
export type ArgsArray = OneArgArray | NoArgsArray;

// @public
export interface Auth {
    getUserIdentity(): Promise<UserIdentity | null>;
}

// @public
export interface CronJob {
    // Warning: (ae-forgotten-export) The symbol "JSONValue" needs to be exported by the entry point index.d.ts
    //
    // (undocumented)
    args: JSONValue;
    // (undocumented)
    name: string;
    // Warning: (ae-forgotten-export) The symbol "Schedule" needs to be exported by the entry point index.d.ts
    //
    // (undocumented)
    schedule: Schedule;
}

// @public
export const cronJobs: () => Crons;

// @public
export class Crons {
    constructor();
    // Warning: (ae-forgotten-export) The symbol "CronString" needs to be exported by the entry point index.d.ts
    cron<FuncRef extends SchedulableFunctionReference>(cronIdentifier: string, cron: CronString, functionReference: FuncRef, ...args: OptionalRestArgs<FuncRef>): void;
    // (undocumented)
    crons: Record<string, CronJob>;
    // Warning: (ae-forgotten-export) The symbol "Daily" needs to be exported by the entry point index.d.ts
    daily<FuncRef extends SchedulableFunctionReference>(cronIdentifier: string, schedule: Daily, functionReference: FuncRef, ...args: OptionalRestArgs<FuncRef>): void;
    // Warning: (ae-forgotten-export) The symbol "Hourly" needs to be exported by the entry point index.d.ts
    hourly<FuncRef extends SchedulableFunctionReference>(cronIdentifier: string, schedule: Hourly, functionReference: FuncRef, ...args: OptionalRestArgs<FuncRef>): void;
    // Warning: (ae-forgotten-export) The symbol "Interval" needs to be exported by the entry point index.d.ts
    interval<FuncRef extends SchedulableFunctionReference>(cronIdentifier: string, schedule: Interval, functionReference: FuncRef, ...args: OptionalRestArgs<FuncRef>): void;
    // (undocumented)
    isCrons: true;
    // Warning: (ae-forgotten-export) The symbol "Monthly" needs to be exported by the entry point index.d.ts
    monthly<FuncRef extends SchedulableFunctionReference>(cronIdentifier: string, schedule: Monthly, functionReference: FuncRef, ...args: OptionalRestArgs<FuncRef>): void;
    // Warning: (ae-forgotten-export) The symbol "Weekly" needs to be exported by the entry point index.d.ts
    weekly<FuncRef extends SchedulableFunctionReference>(cronIdentifier: string, schedule: Weekly, functionReference: FuncRef, ...args: OptionalRestArgs<FuncRef>): void;
}

// @public
export type Cursor = string;

// @public @deprecated (undocumented)
export interface DatabaseReader<DataModel extends GenericDataModel> {
    // Warning: (ae-forgotten-export) The symbol "Id" needs to be exported by the entry point index.d.ts
    get<TableName extends TableNamesInDataModel<DataModel>>(id: Id<TableName>): Promise<DocumentByName<DataModel, TableName> | null>;
    normalizeId<TableName extends TableNamesInDataModel<DataModel>>(tableName: TableName, id: string): Id<TableName> | null;
    query<TableName extends TableNamesInDataModel<DataModel>>(tableName: TableName): QueryInitializer<NamedTableInfo<DataModel, TableName>>;
}

// @public @deprecated (undocumented)
export interface DatabaseWriter<DataModel extends GenericDataModel> extends GenericDatabaseReader<DataModel> {
    delete(id: Id<TableNamesInDataModel<DataModel>>): Promise<void>;
    insert<TableName extends TableNamesInDataModel<DataModel>>(table: TableName, value: WithoutSystemFields<DocumentByName<DataModel, TableName>>): Promise<Id<TableName>>;
    patch<TableName extends TableNamesInDataModel<DataModel>>(id: Id<TableName>, value: Partial<DocumentByName<DataModel, TableName>>): Promise<void>;
    // Warning: (ae-forgotten-export) The symbol "WithOptionalSystemFields" needs to be exported by the entry point index.d.ts
    replace<TableName extends TableNamesInDataModel<DataModel>>(id: Id<TableName>, value: WithOptionalSystemFields<DocumentByName<DataModel, TableName>>): Promise<void>;
}

// Warning: (ae-forgotten-export) The symbol "MaybeMakeLooseDataModel" needs to be exported by the entry point index.d.ts
//
// @public
export type DataModelFromSchemaDefinition<SchemaDef extends SchemaDefinition<any, boolean>> = MaybeMakeLooseDataModel<{
    [TableName in keyof SchemaDef["tables"] & string]: SchemaDef["tables"][TableName] extends TableDefinition<infer Document, infer FieldPaths, infer Indexes, infer SearchIndexes, infer VectorIndexes> ? {
        document: Expand<IdField<TableName> & Document>;
        fieldPaths: keyof IdField<TableName> | FieldPaths;
        indexes: Expand<Indexes & SystemIndexes>;
        searchIndexes: SearchIndexes;
        vectorIndexes: VectorIndexes;
    } : never;
}, SchemaDef["strictTableNameTypes"]>;

// @public
export type DefaultFunctionArgs = Record<string, unknown>;

// @public
export function defineSchema<Schema extends GenericSchema, StrictTableNameTypes extends boolean = true>(schema: Schema, options?: DefineSchemaOptions<StrictTableNameTypes>): SchemaDefinition<Schema, StrictTableNameTypes>;

// @public
export interface DefineSchemaOptions<StrictTableNameTypes extends boolean> {
    schemaValidation?: boolean;
    strictTableNameTypes?: StrictTableNameTypes;
}

// Warning: (ae-forgotten-export) The symbol "Validator" needs to be exported by the entry point index.d.ts
// Warning: (ae-forgotten-export) The symbol "ExtractDocument" needs to be exported by the entry point index.d.ts
// Warning: (ae-forgotten-export) The symbol "ExtractFieldPaths" needs to be exported by the entry point index.d.ts
//
// @public
export function defineTable<DocumentSchema extends Validator<Record<string, any>, false, any>>(documentSchema: DocumentSchema): TableDefinition<ExtractDocument<DocumentSchema>, ExtractFieldPaths<DocumentSchema>>;

// Warning: (ae-forgotten-export) The symbol "ObjectValidator" needs to be exported by the entry point index.d.ts
//
// @public
export function defineTable<DocumentSchema extends Record<string, Validator<any, any, any>>>(documentSchema: DocumentSchema): TableDefinition<ExtractDocument<ObjectValidator<DocumentSchema>>, ExtractFieldPaths<ObjectValidator<DocumentSchema>>>;

// @public
export type DocumentByInfo<TableInfo extends GenericTableInfo> = TableInfo["document"];

// @public
export type DocumentByName<DataModel extends GenericDataModel, TableName extends TableNamesInDataModel<DataModel>> = DataModel[TableName]["document"];

// Warning: (ae-forgotten-export) The symbol "Value" needs to be exported by the entry point index.d.ts
//
// @public
export abstract class Expression<T extends Value | undefined> {
}

// @public
export type ExpressionOrValue<T extends Value | undefined> = Expression<T> | T;

// @public
export type FieldPaths<TableInfo extends GenericTableInfo> = TableInfo["fieldPaths"];

// @public
export type FieldTypeFromFieldPath<Document extends GenericDocument, FieldPath extends string> = FieldPath extends `${infer First}.${infer Second}` ? First extends keyof Document ? Document[First] extends GenericDocument ? FieldTypeFromFieldPath<Document[First], Second> : undefined : undefined : FieldPath extends keyof Document ? Document[FieldPath] : undefined;

// @public
export type FileMetadata = {
    storageId: StorageId;
    sha256: string;
    size: number;
    contentType: string | null;
};

// @public
export type FilterApi<API, Predicate> = Expand<{
    [mod in keyof API as API[mod] extends Predicate ? mod : API[mod] extends FunctionReference<any, any, any, any> ? never : FilterApi<API[mod], Predicate> extends Record<string, never> ? never : mod]: API[mod] extends Predicate ? API[mod] : FilterApi<API[mod], Predicate>;
}>;

// @public
export function filterApi<API, Predicate>(api: API): FilterApi<API, Predicate>;

// @public
export interface FilterBuilder<TableInfo extends GenericTableInfo> {
    // Warning: (ae-forgotten-export) The symbol "NumericValue" needs to be exported by the entry point index.d.ts
    add<T extends NumericValue>(l: ExpressionOrValue<T>, r: ExpressionOrValue<T>): Expression<T>;
    and(...exprs: Array<ExpressionOrValue<boolean>>): Expression<boolean>;
    div<T extends NumericValue>(l: ExpressionOrValue<T>, r: ExpressionOrValue<T>): Expression<T>;
    eq<T extends Value | undefined>(l: ExpressionOrValue<T>, r: ExpressionOrValue<T>): Expression<boolean>;
    field<FieldPath extends FieldPaths<TableInfo>>(fieldPath: FieldPath): Expression<FieldTypeFromFieldPath<DocumentByInfo<TableInfo>, FieldPath>>;
    gt<T extends Value>(l: ExpressionOrValue<T>, r: ExpressionOrValue<T>): Expression<boolean>;
    gte<T extends Value>(l: ExpressionOrValue<T>, r: ExpressionOrValue<T>): Expression<boolean>;
    lt<T extends Value>(l: ExpressionOrValue<T>, r: ExpressionOrValue<T>): Expression<boolean>;
    lte<T extends Value>(l: ExpressionOrValue<T>, r: ExpressionOrValue<T>): Expression<boolean>;
    mod<T extends NumericValue>(l: ExpressionOrValue<T>, r: ExpressionOrValue<T>): Expression<T>;
    mul<T extends NumericValue>(l: ExpressionOrValue<T>, r: ExpressionOrValue<T>): Expression<T>;
    neg<T extends NumericValue>(x: ExpressionOrValue<T>): Expression<T>;
    neq<T extends Value | undefined>(l: ExpressionOrValue<T>, r: ExpressionOrValue<T>): Expression<boolean>;
    not(x: ExpressionOrValue<boolean>): Expression<boolean>;
    or(...exprs: Array<ExpressionOrValue<boolean>>): Expression<boolean>;
    sub<T extends NumericValue>(l: ExpressionOrValue<T>, r: ExpressionOrValue<T>): Expression<T>;
}

// @public
export type FunctionArgs<FuncRef extends AnyFunctionReference> = FuncRef["_args"];

// @public
export type FunctionReference<Type extends FunctionType, Visibility extends FunctionVisibility = "public", Args extends DefaultFunctionArgs = any, ReturnType = any> = {
    _type: Type;
    _visibility: Visibility;
    _args: Args;
    _returnType: ReturnType;
};

// @public
export type FunctionReturnType<FuncRef extends AnyFunctionReference> = FuncRef["_returnType"];

// @public
export type FunctionType = "query" | "mutation" | "action";

// @public
export type FunctionVisibility = "public" | "internal";

// @public
export interface GenericActionCtx<DataModel extends GenericDataModel> {
    auth: Auth;
    runAction<Action extends FunctionReference<"action", "public" | "internal">>(action: Action, ...args: OptionalRestArgs<Action>): Promise<FunctionReturnType<Action>>;
    runMutation<Mutation extends FunctionReference<"mutation", "public" | "internal">>(mutation: Mutation, ...args: OptionalRestArgs<Mutation>): Promise<FunctionReturnType<Mutation>>;
    runQuery<Query extends FunctionReference<"query", "public" | "internal">>(query: Query, ...args: OptionalRestArgs<Query>): Promise<FunctionReturnType<Query>>;
    scheduler: Scheduler;
    storage: StorageActionWriter;
}

// @public
export interface GenericDatabaseReader<DataModel extends GenericDataModel> extends DatabaseReader<DataModel> {
}

// @public
export interface GenericDatabaseWriter<DataModel extends GenericDataModel> extends DatabaseWriter<DataModel> {
}

// @public
export type GenericDataModel = Record<string, GenericTableInfo>;

// @public
export type GenericDocument = Record<string, Value>;

// @public
export type GenericFieldPaths = string;

// @public
export type GenericIndexFields = string[];

// @public
export interface GenericMutationCtx<DataModel extends GenericDataModel> {
    auth: Auth;
    db: GenericDatabaseWriter<DataModel>;
    scheduler: Scheduler;
    storage: StorageWriter;
}

// @public
export interface GenericQueryCtx<DataModel extends GenericDataModel> {
    auth: Auth;
    db: GenericDatabaseReader<DataModel>;
    storage: StorageReader;
}

// @public
export type GenericSchema = Record<string, TableDefinition>;

// @public
export type GenericSearchIndexConfig = {
    searchField: string;
    filterFields: string;
};

// @public
export type GenericTableIndexes = Record<string, GenericIndexFields>;

// @public
export type GenericTableInfo = {
    document: GenericDocument;
    fieldPaths: GenericFieldPaths;
    indexes: GenericTableIndexes;
    searchIndexes: GenericTableSearchIndexes;
    vectorIndexes: GenericTableVectorIndexes;
};

// @public
export type GenericTableSearchIndexes = Record<string, GenericSearchIndexConfig>;

// @public
export function getFunctionName(functionReference: AnyFunctionReference): string;

// @public
export type HttpActionBuilder = (func: (ctx: GenericActionCtx<any>, request: Request) => Promise<Response>) => PublicHttpAction;

// @public
export const httpActionGeneric: (func: (ctx: ActionCtx<GenericDataModel>, request: Request) => Promise<Response>) => PublicHttpAction;

// @public
export class HttpRouter {
    // (undocumented)
    exactRoutes: Map<string, Map<RoutableMethod, PublicHttpAction>>;
    getRoutes: () => (readonly [string, "GET" | "POST" | "PUT" | "DELETE" | "OPTIONS" | "PATCH", (...args: any[]) => any])[];
    // (undocumented)
    isRouter: boolean;
    lookup: (path: string, method: RoutableMethod | "HEAD") => Readonly<[PublicHttpAction, RoutableMethod, string]> | null;
    // (undocumented)
    prefixRoutes: Map<RoutableMethod, Map<string, PublicHttpAction>>;
    // Warning: (ae-forgotten-export) The symbol "RouteSpec" needs to be exported by the entry point index.d.ts
    route: (spec: RouteSpec) => void;
    runRequest: (argsStr: string) => Promise<string>;
}

// @public
export const httpRouter: () => HttpRouter;

// @public
export type Indexes<TableInfo extends GenericTableInfo> = TableInfo["indexes"];

// @public
export type IndexNames<TableInfo extends GenericTableInfo> = keyof Indexes<TableInfo>;

// @public
export abstract class IndexRange {
}

// Warning: (ae-forgotten-export) The symbol "LowerBoundIndexRangeBuilder" needs to be exported by the entry point index.d.ts
//
// @public
export interface IndexRangeBuilder<Document extends GenericDocument, IndexFields extends GenericIndexFields, FieldNum extends number = 0> extends LowerBoundIndexRangeBuilder<Document, IndexFields[FieldNum]> {
    // Warning: (ae-forgotten-export) The symbol "NextIndexRangeBuilder" needs to be exported by the entry point index.d.ts
    eq(fieldName: IndexFields[FieldNum], value: FieldTypeFromFieldPath<Document, IndexFields[FieldNum]>): NextIndexRangeBuilder<Document, IndexFields, FieldNum>;
}

// @public
export const internalActionGeneric: ActionBuilder<any, "internal">;

// @public
export const internalMutationGeneric: MutationBuilder<any, "internal">;

// @public
export const internalQueryGeneric: QueryBuilder<any, "internal">;

// @public
export function makeFunctionReference<type extends FunctionType, args extends DefaultFunctionArgs = any, ret = any>(name: string): FunctionReference<type, "public", args, ret>;

// @public
export type MutationBuilder<DataModel extends GenericDataModel, Visibility extends FunctionVisibility> = {
    <Output, ArgsValidator extends PropertyValidators>(func: ValidatedFunction<GenericMutationCtx<DataModel>, ArgsValidator, Output>): RegisteredMutation<Visibility, ObjectType<ArgsValidator>, Output>;
    <Output, Args extends ArgsArray = OneArgArray>(func: UnvalidatedFunction<GenericMutationCtx<DataModel>, Args, Output>): RegisteredMutation<Visibility, ArgsArrayToObject<Args>, Output>;
};

// @public @deprecated
export interface MutationCtx<DataModel extends GenericDataModel> extends GenericMutationCtx<DataModel> {
}

// @public
export const mutationGeneric: MutationBuilder<any, "public">;

// @public
export type NamedIndex<TableInfo extends GenericTableInfo, IndexName extends IndexNames<TableInfo>> = Indexes<TableInfo>[IndexName];

// @public
export type NamedSearchIndex<TableInfo extends GenericTableInfo, IndexName extends SearchIndexNames<TableInfo>> = SearchIndexes<TableInfo>[IndexName];

// @public
export type NamedTableInfo<DataModel extends GenericDataModel, TableName extends keyof DataModel> = DataModel[TableName];

// @public
export type OptionalRestArgs<FuncRef extends AnyFunctionReference> = FuncRef["_args"] extends EmptyObject ? [args?: EmptyObject] : [args: FuncRef["_args"]];

// @public
export interface OrderedQuery<TableInfo extends GenericTableInfo> extends AsyncIterable<DocumentByInfo<TableInfo>> {
    collect(): Promise<Array<DocumentByInfo<TableInfo>>>;
    filter(predicate: (q: FilterBuilder<TableInfo>) => ExpressionOrValue<boolean>): this;
    first(): Promise<DocumentByInfo<TableInfo> | null>;
    paginate(paginationOpts: PaginationOptions): Promise<PaginationResult<DocumentByInfo<TableInfo>>>;
    take(n: number): Promise<Array<DocumentByInfo<TableInfo>>>;
    unique(): Promise<DocumentByInfo<TableInfo> | null>;
}

// @public
export interface PaginationOptions {
    cursor: Cursor | null;
    numItems: number;
}

// @public
export const paginationOptsValidator: ObjectValidator<    {
numItems: Validator<number, false, never>;
cursor: Validator<string | null, false, never>;
id: Validator<number | undefined, true, never>;
}>;

// @public
export interface PaginationResult<T> {
    continueCursor: Cursor;
    isDone: boolean;
    page: T[];
}

// @public
export type PartialApi<API> = {
    [mod in keyof API]?: API[mod] extends FunctionReference<any, any, any, any> ? API[mod] : PartialApi<API[mod]>;
};

// @public
export type PublicHttpAction = {
    (ctx: GenericActionCtx<any>, request: Request): Response;
    isHttp: true;
    isRegistered?: true;
};

// @public
export interface Query<TableInfo extends GenericTableInfo> extends OrderedQuery<TableInfo> {
    order(order: "asc" | "desc"): OrderedQuery<TableInfo>;
}

// @public
export type QueryBuilder<DataModel extends GenericDataModel, Visibility extends FunctionVisibility> = {
    <Output, ArgsValidator extends PropertyValidators>(func: ValidatedFunction<GenericQueryCtx<DataModel>, ArgsValidator, Output>): RegisteredQuery<Visibility, ObjectType<ArgsValidator>, Output>;
    <Output, Args extends ArgsArray = OneArgArray>(func: UnvalidatedFunction<GenericQueryCtx<DataModel>, Args, Output>): RegisteredQuery<Visibility, ArgsArrayToObject<Args>, Output>;
};

// @public @deprecated
export interface QueryCtx<DataModel extends GenericDataModel> extends GenericQueryCtx<DataModel> {
}

// @public
export const queryGeneric: QueryBuilder<any, "public">;

// @public
export interface QueryInitializer<TableInfo extends GenericTableInfo> extends Query<TableInfo> {
    fullTableScan(): Query<TableInfo>;
    withIndex<IndexName extends IndexNames<TableInfo>>(indexName: IndexName, indexRange?: (q: IndexRangeBuilder<DocumentByInfo<TableInfo>, NamedIndex<TableInfo, IndexName>>) => IndexRange): Query<TableInfo>;
    withSearchIndex<IndexName extends SearchIndexNames<TableInfo>>(indexName: IndexName, searchFilter: (q: SearchFilterBuilder<DocumentByInfo<TableInfo>, NamedSearchIndex<TableInfo, IndexName>>) => SearchFilter): OrderedQuery<TableInfo>;
}

// Warning: (ae-forgotten-export) The symbol "VisibilityProperties" needs to be exported by the entry point index.d.ts
//
// @public
export type RegisteredAction<Visibility extends FunctionVisibility, Args extends DefaultFunctionArgs, Output> = {
    (ctx: GenericActionCtx<any>, args: Args): Output;
    isConvexFunction: true;
    isAction: true;
    isRegistered?: true;
} & VisibilityProperties<Visibility>;

// @public
export type RegisteredMutation<Visibility extends FunctionVisibility, Args extends DefaultFunctionArgs, Output> = {
    (ctx: GenericMutationCtx<any>, args: Args): Output;
    isConvexFunction: true;
    isMutation: true;
    isRegistered?: true;
} & VisibilityProperties<Visibility>;

// @public
export type RegisteredQuery<Visibility extends FunctionVisibility, Args extends DefaultFunctionArgs, Output> = {
    (ctx: GenericQueryCtx<any>, args: Args): Output;
    isConvexFunction: true;
    isQuery: true;
    isRegistered?: true;
} & VisibilityProperties<Visibility>;

// @public
export const ROUTABLE_HTTP_METHODS: readonly ["GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"];

// @public
export type RoutableMethod = (typeof ROUTABLE_HTTP_METHODS)[number];

// @public
export type SchedulableFunctionReference = FunctionReference<"mutation" | "action", "public" | "internal">;

// @public
export interface Scheduler {
    runAfter<FuncRef extends SchedulableFunctionReference>(delayMs: number, functionReference: FuncRef, ...args: OptionalRestArgs<FuncRef>): Promise<void>;
    runAt<FuncRef extends SchedulableFunctionReference>(timestamp: number | Date, functionReference: FuncRef, ...args: OptionalRestArgs<FuncRef>): Promise<void>;
}

// @public
export class SchemaDefinition<Schema extends GenericSchema, StrictTableTypes extends boolean> {
    // (undocumented)
    strictTableNameTypes: StrictTableTypes;
    // (undocumented)
    tables: Schema;
}

// @public
export abstract class SearchFilter {
}

// @public
export interface SearchFilterBuilder<Document extends GenericDocument, SearchIndexConfig extends GenericSearchIndexConfig> {
    search(fieldName: SearchIndexConfig["searchField"], query: string): SearchFilterFinalizer<Document, SearchIndexConfig>;
}

// @public
export interface SearchFilterFinalizer<Document extends GenericDocument, SearchIndexConfig extends GenericSearchIndexConfig> extends SearchFilter {
    eq<FieldName extends SearchIndexConfig["filterFields"]>(fieldName: FieldName, value: FieldTypeFromFieldPath<Document, FieldName>): SearchFilterFinalizer<Document, SearchIndexConfig>;
}

// @public
export interface SearchIndexConfig<SearchField extends string, FilterFields extends string> {
    filterFields?: FilterFields[];
    searchField: SearchField;
}

// @public
export type SearchIndexes<TableInfo extends GenericTableInfo> = TableInfo["searchIndexes"];

// @public
export type SearchIndexNames<TableInfo extends GenericTableInfo> = keyof SearchIndexes<TableInfo>;

// @public
export interface StorageActionWriter extends StorageWriter {
    get(storageId: StorageId): Promise<Blob | null>;
    store(blob: Blob, options?: {
        sha256?: string;
    }): Promise<StorageId>;
}

// @public
export type StorageId = string;

// @public
export interface StorageReader {
    getMetadata(storageId: StorageId): Promise<FileMetadata | null>;
    getUrl(storageId: StorageId): Promise<string | null>;
}

// @public
export interface StorageWriter extends StorageReader {
    delete(storageId: StorageId): Promise<void>;
    generateUploadUrl(): Promise<string>;
}

// @public
export class TableDefinition<Document extends GenericDocument = GenericDocument, FieldPaths extends string = string, Indexes extends GenericTableIndexes = {}, SearchIndexes extends GenericTableSearchIndexes = {}, VectorIndexes extends GenericTableVectorIndexes = {}> {
    // Warning: (ae-forgotten-export) The symbol "IndexTiebreakerField" needs to be exported by the entry point index.d.ts
    index<IndexName extends string, FirstFieldPath extends FieldPaths, RestFieldPaths extends FieldPaths[]>(name: IndexName, fields: [FirstFieldPath, ...RestFieldPaths]): TableDefinition<Document, FieldPaths, Expand<Indexes & Record<IndexName, [
    FirstFieldPath,
    ...RestFieldPaths,
    IndexTiebreakerField
    ]>>, SearchIndexes, VectorIndexes>;
    searchIndex<IndexName extends string, SearchField extends FieldPaths, FilterFields extends FieldPaths = never>(name: IndexName, indexConfig: Expand<SearchIndexConfig<SearchField, FilterFields>>): TableDefinition<Document, FieldPaths, Indexes, Expand<SearchIndexes & Record<IndexName, {
        searchField: SearchField;
        filterFields: FilterFields;
    }>>, VectorIndexes>;
}

// @public
export type TableNamesInDataModel<DataModel extends GenericDataModel> = keyof DataModel & string;

// @public
export type UnvalidatedFunction<Ctx, Args extends ArgsArray, Output> = ((ctx: Ctx, ...args: Args) => Output) | {
    handler: (ctx: Ctx, ...args: Args) => Output;
};

// @public
export interface UserIdentity {
    // (undocumented)
    readonly address?: string;
    // (undocumented)
    readonly birthday?: string;
    // (undocumented)
    readonly email?: string;
    // (undocumented)
    readonly emailVerified?: boolean;
    // (undocumented)
    readonly familyName?: string;
    // (undocumented)
    readonly gender?: string;
    // (undocumented)
    readonly givenName?: string;
    readonly issuer: string;
    // (undocumented)
    readonly language?: string;
    // (undocumented)
    readonly name?: string;
    // (undocumented)
    readonly nickname?: string;
    // (undocumented)
    readonly phoneNumber?: string;
    // (undocumented)
    readonly phoneNumberVerified?: boolean;
    // (undocumented)
    readonly pictureUrl?: string;
    // (undocumented)
    readonly preferredUsername?: string;
    // (undocumented)
    readonly profileUrl?: string;
    readonly subject: string;
    // (undocumented)
    readonly timezone?: string;
    readonly tokenIdentifier: string;
    // (undocumented)
    readonly updatedAt?: string;
}

// @public
export interface ValidatedFunction<Ctx, ArgsValidator extends PropertyValidators, Output> {
    args: ArgsValidator;
    handler: (ctx: Ctx, args: ObjectType<ArgsValidator>) => Output;
}

// Warning: (ae-forgotten-export) The symbol "BetterOmit" needs to be exported by the entry point index.d.ts
// Warning: (ae-forgotten-export) The symbol "SystemFields" needs to be exported by the entry point index.d.ts
//
// @public
export type WithoutSystemFields<Document extends GenericDocument> = Expand<BetterOmit<Document, keyof SystemFields | "_id">>;

// Warnings were encountered during analysis:
//
// src/server/data_model.ts:106:3 - (ae-forgotten-export) The symbol "GenericTableVectorIndexes" needs to be exported by the entry point index.d.ts
// src/server/registration.ts:527:3 - (ae-forgotten-export) The symbol "PropertyValidators" needs to be exported by the entry point index.d.ts
// src/server/registration.ts:527:3 - (ae-forgotten-export) The symbol "ObjectType" needs to be exported by the entry point index.d.ts
// src/server/registration.ts:531:3 - (ae-forgotten-export) The symbol "OneArgArray" needs to be exported by the entry point index.d.ts
// src/server/registration.ts:531:3 - (ae-forgotten-export) The symbol "ArgsArrayToObject" needs to be exported by the entry point index.d.ts
// src/server/schema.ts:521:11 - (ae-forgotten-export) The symbol "Expand" needs to be exported by the entry point index.d.ts
// src/server/schema.ts:521:11 - (ae-forgotten-export) The symbol "IdField" needs to be exported by the entry point index.d.ts
// src/server/schema.ts:523:11 - (ae-forgotten-export) The symbol "SystemIndexes" needs to be exported by the entry point index.d.ts

// (No @packageDocumentation comment for this package)

```