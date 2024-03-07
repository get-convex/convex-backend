import {
  Auth,
  GenericDatabaseReader,
  GenericDatabaseWriter,
  StorageActionWriter,
  StorageReader,
  StorageWriter,
} from "./index.js";
import {
  FunctionReference,
  FunctionReturnType,
  OptionalRestArgs,
} from "../server/api.js";
import { ObjectType, PropertyValidators } from "../values/validator.js";
import { Id } from "../values/value.js";
import {
  GenericDataModel,
  NamedTableInfo,
  TableNamesInDataModel,
  VectorIndexNames,
} from "./data_model.js";
import { Scheduler } from "./scheduler.js";
import { VectorSearchQuery } from "./vector_search.js";
import { Expand } from "../type_utils.js";

/**
 * A set of services for use within Convex mutation functions.
 *
 * The mutation context is passed as the first argument to any Convex mutation
 * function run on the server.
 *
 * If you're using code generation, use the `MutationCtx` type in
 * `convex/_generated/server.d.ts` which is typed for your data model.
 *
 * @public
 */
export interface GenericMutationCtx<DataModel extends GenericDataModel> {
  /**
   * A utility for reading and writing data in the database.
   */
  db: GenericDatabaseWriter<DataModel>;

  /**
   * Information about the currently authenticated user.
   */
  auth: Auth;

  /**
   * A utility for reading and writing files in storage.
   */
  storage: StorageWriter;

  /**
   * A utility for scheduling Convex functions to run in the future.
   */
  scheduler: Scheduler;
}

/**
 * If you're using code generation, use the `MutationCtx` type in
 * `convex/_generated/server.d.ts` which is typed for your data model.
 *
 * If you need an unparameterized MutationCtx use GenericMutationCtx.
 *
 * @public
 * @deprecated
 */
export interface MutationCtx<DataModel extends GenericDataModel>
  extends GenericMutationCtx<DataModel> {}

/**
 * A set of services for use within Convex query functions.
 *
 * The query context is passed as the first argument to any Convex query
 * function run on the server.
 *
 * This differs from the {@link MutationCtx} because all of the services are
 * read-only.
 *
 *
 * @public
 */
export interface GenericQueryCtx<DataModel extends GenericDataModel> {
  /**
   * A utility for reading data in the database.
   */
  db: GenericDatabaseReader<DataModel>;

  /**
   * Information about the currently authenticated user.
   */
  auth: Auth;

  /**
   * A utility for reading files in storage.
   */
  storage: StorageReader;
}

/**
 * If you're using code generation, use the `QueryCtx` type in
 * `convex/_generated/server.d.ts` which is typed for your data model.
 *
 * If you need an unparameterized QueryCtx use GenericQueryCtx.
 *
 * @public
 * @deprecated
 */
export interface QueryCtx<DataModel extends GenericDataModel>
  extends GenericQueryCtx<DataModel> {}

/**
 * A set of services for use within Convex action functions.
 *
 * The context is passed as the first argument to any Convex action
 * run on the server.
 *
 * If you're using code generation, use the `ActionCtx` type in
 * `convex/_generated/server.d.ts` which is typed for your data model.
 *
 * @public
 */
export interface GenericActionCtx<DataModel extends GenericDataModel> {
  /**
   * Run the Convex query with the given name and arguments.
   *
   * Consider using an {@link internalQuery} to prevent users from calling the
   * query directly.
   *
   * @param query - A {@link FunctionReference} for the query to run.
   * @param args - The arguments to the query function.
   * @returns A promise of the query's result.
   */
  runQuery<Query extends FunctionReference<"query", "public" | "internal">>(
    query: Query,
    ...args: OptionalRestArgs<Query>
  ): Promise<FunctionReturnType<Query>>;

  /**
   * Run the Convex mutation with the given name and arguments.
   *
   * Consider using an {@link internalMutation} to prevent users from calling
   * the mutation directly.
   *
   * @param mutation - A {@link FunctionReference} for the mutation to run.
   * @param args - The arguments to the mutation function.
   * @returns A promise of the mutation's result.
   */
  runMutation<
    Mutation extends FunctionReference<"mutation", "public" | "internal">,
  >(
    mutation: Mutation,
    ...args: OptionalRestArgs<Mutation>
  ): Promise<FunctionReturnType<Mutation>>;

  /**
   * Run the Convex action with the given name and arguments.
   *
   * Consider using an {@link internalAction} to prevent users from calling the
   * action directly.
   *
   * @param action - A {@link FunctionReference} for the action to run.
   * @param args - The arguments to the action function.
   * @returns A promise of the action's result.
   */
  runAction<Action extends FunctionReference<"action", "public" | "internal">>(
    action: Action,
    ...args: OptionalRestArgs<Action>
  ): Promise<FunctionReturnType<Action>>;

  /**
   * A utility for scheduling Convex functions to run in the future.
   */
  scheduler: Scheduler;

  /**
   * Information about the currently authenticated user.
   */
  auth: Auth;

  /**
   * A utility for reading and writing files in storage.
   */
  storage: StorageActionWriter;

  /**
   * Run a vector search on the given table and index.
   *
   * @param tableName - The name of the table to query.
   * @param indexName - The name of the vector index on the table to query.
   * @param query - A {@link VectorSearchQuery} containing the vector to query,
   * the number of results to return, and any filters.
   * @returns A promise of IDs and scores for the documents with the nearest
   * vectors
   */
  vectorSearch<
    TableName extends TableNamesInDataModel<DataModel>,
    IndexName extends VectorIndexNames<NamedTableInfo<DataModel, TableName>>,
  >(
    tableName: TableName,
    indexName: IndexName,
    query: Expand<
      VectorSearchQuery<NamedTableInfo<DataModel, TableName>, IndexName>
    >,
  ): Promise<Array<{ _id: Id<TableName>; _score: number }>>;
}

/**
 * If you're using code generation, use the `ActionCtx` type in
 * `convex/_generated/server.d.ts` which is typed for your data model.
 *
 * If you need an unparameterized ActionCtx use GenericActionCtx.
 *
 * @public
 * @deprecated
 */
export interface ActionCtx<
  DataModel extends GenericDataModel = GenericDataModel,
> extends GenericActionCtx<DataModel> {}

/**
 * The default arguments type for a Convex query, mutation, or action function.
 *
 * Convex functions always take an arguments object that maps the argument
 * names to their values.
 *
 * @public
 */
export type DefaultFunctionArgs = Record<string, unknown>;

/**
 * The arguments array for a function that takes arguments.
 *
 * This is an array of a single {@link DefaultFunctionArgs} element.
 */
type OneArgArray<ArgsObject extends DefaultFunctionArgs = DefaultFunctionArgs> =
  [ArgsObject];

/**
 * The arguments to a function that takes no arguments (just an empty array).
 */
type NoArgsArray = [];

/**
 * An array of arguments to a Convex function.
 *
 * Convex functions can take either a single {@link DefaultFunctionArgs} object or no
 * args at all.
 *
 * @public
 */
export type ArgsArray = OneArgArray | NoArgsArray;

/**
 * A type for the empty object `{}`.
 *
 * Note that we don't use `type EmptyObject = {}` because that matches every object.
 */
export type EmptyObject = Record<string, never>;

/**
 * Convert an {@link ArgsArray} into a single object type.
 *
 * Empty arguments arrays are converted to {@link EmptyObject}.
 */
type ArgsArrayToObject<Args extends ArgsArray> =
  Args extends OneArgArray<infer ArgsObject> ? ArgsObject : EmptyObject;

/**
 * A type representing the visibility of a Convex function.
 *
 * @public
 */
export type FunctionVisibility = "public" | "internal";

/**
 * Given a {@link FunctionVisibility}, should this function have `isPublic: true`
 * or `isInternal: true`?
 */
type VisibilityProperties<Visiblity extends FunctionVisibility> =
  Visiblity extends "public"
    ? {
        isPublic: true;
      }
    : {
        isInternal: true;
      };

/**
 * A mutation function that is part of this app.
 *
 * You can create a mutation by wrapping your function in
 * {@link mutationGeneric} or {@link internalMutationGeneric} and exporting it.
 *
 * @public
 */
export type RegisteredMutation<
  Visibility extends FunctionVisibility,
  Args extends DefaultFunctionArgs,
  Output,
> = {
  (ctx: GenericMutationCtx<any>, args: Args): Output;

  isConvexFunction: true;
  isMutation: true;
  isRegistered?: true;

  /** @internal */
  invokeMutation(argsStr: string): Promise<string>;

  /** @internal */
  exportArgs(): string;
} & VisibilityProperties<Visibility>;

/**
 * A query function that is part of this app.
 *
 * You can create a query by wrapping your function in
 * {@link queryGeneric} or {@link internalQueryGeneric} and exporting it.
 *
 * @public
 */
export type RegisteredQuery<
  Visibility extends FunctionVisibility,
  Args extends DefaultFunctionArgs,
  Output,
> = {
  (ctx: GenericQueryCtx<any>, args: Args): Output;

  isConvexFunction: true;
  isQuery: true;
  isRegistered?: true;

  /** @internal */
  invokeQuery(argsStr: string): Promise<string>;

  /** @internal */
  exportArgs(): string;
} & VisibilityProperties<Visibility>;

/**
 * An action that is part of this app.
 *
 * You can create an action by wrapping your function in
 * {@link actionGeneric} or {@link internalActionGeneric} and exporting it.
 *
 * @public
 */
export type RegisteredAction<
  Visibility extends FunctionVisibility,
  Args extends DefaultFunctionArgs,
  Output,
> = {
  (ctx: GenericActionCtx<any>, args: Args): Output;

  isConvexFunction: true;
  isAction: true;
  isRegistered?: true;

  /** @internal */
  invokeAction(requestId: string, argsStr: string): Promise<string>;

  /** @internal */
  exportArgs(): string;
} & VisibilityProperties<Visibility>;

/**
 * An HTTP action that is part of this app's public API.
 *
 * You can create public HTTP actions by wrapping your function in
 * {@link httpActionGeneric} and exporting it.
 *
 * @public
 */
export type PublicHttpAction = {
  (ctx: GenericActionCtx<any>, request: Request): Response;
  isHttp: true;
  isRegistered?: true;

  /** @internal */
  invokeHttpAction(request: Request): Promise<Response>;
};

/**
 * The definition of a Convex query, mutation, or action function without
 * argument validation.
 *
 * Convex functions always take a context object as their first argument
 * and an (optional) args object as their second argument.
 *
 * This can be written as a function like:
 * ```js
 * import { query } from "./_generated/server";
 *
 * export const func = query(({ db }, { arg }) => {...});
 * ```
 * or as an object like:
 *
 * ```js
 * import { query } from "./_generated/server";
 *
 * export const func = query({
 *   handler: ({ db }, { arg }) => {...},
 * });
 * ```
 * See {@link ValidatedFunction} to add argument validation.
 *
 * @public
 */
export type UnvalidatedFunction<Ctx, Args extends ArgsArray, Output> =
  | ((ctx: Ctx, ...args: Args) => Output)
  | {
      handler: (ctx: Ctx, ...args: Args) => Output;
    };

/**
 * The definition of a Convex query, mutation, or action function with argument
 * validation.
 *
 * Argument validation allows you to assert that the arguments to this function
 * are the expected type.
 *
 * Example:
 *
 * ```js
 * import { query } from "./_generated/server";
 * import { v } from "convex/values";
 *
 * export const func = query({
 *   args: {
 *     arg: v.string()
 *   },
 *   handler: ({ db }, { arg }) => {...},
 * });
 * ```
 *
 * **For security, argument validation should be added to all public functions in
 * production apps.**
 *
 * See {@link UnvalidatedFunction} for functions without argument validation.
 * @public
 */
export interface ValidatedFunction<
  Ctx,
  ArgsValidator extends PropertyValidators,
  Output,
> {
  /**
   * A validator for the arguments of this function.
   *
   * This is an object mapping argument names to validators constructed with
   * {@link values.v}.
   *
   * ```js
   * import { v } from "convex/values";
   *
   * const args = {
   *   stringArg: v.string(),
   *   optionalNumberArg: v.optional(v.number()),
   * }
   * ```
   */
  args: ArgsValidator;

  /**
   * The implementation of this function.
   *
   * This is a function that takes in the appropriate context and arguments
   * and produces some result.
   *
   * @param ctx - The context object. This is one of {@link QueryCtx},
   * {@link MutationCtx}, or {@link ActionCtx} depending on the function type.
   * @param args - The arguments object for this function. This will match
   * the type defined by the argument validator.
   * @returns
   */
  handler: (ctx: Ctx, args: ObjectType<ArgsValidator>) => Output;
}

/**
 * Internal type helper used by Convex code generation.
 *
 * Used to give {@link mutationGeneric} a type specific to your data model.
 * @public
 */
export type MutationBuilder<
  DataModel extends GenericDataModel,
  Visibility extends FunctionVisibility,
> = {
  <Output, ArgsValidator extends PropertyValidators>(
    func: ValidatedFunction<
      GenericMutationCtx<DataModel>,
      ArgsValidator,
      Output
    >,
  ): RegisteredMutation<Visibility, ObjectType<ArgsValidator>, Output>;

  <Output, Args extends ArgsArray = OneArgArray>(
    func: UnvalidatedFunction<GenericMutationCtx<DataModel>, Args, Output>,
  ): RegisteredMutation<Visibility, ArgsArrayToObject<Args>, Output>;
};

/**
 * Internal type helper used by Convex code generation.
 *
 * Used to give {@link queryGeneric} a type specific to your data model.
 * @public
 */
export type QueryBuilder<
  DataModel extends GenericDataModel,
  Visibility extends FunctionVisibility,
> = {
  <Output, ArgsValidator extends PropertyValidators>(
    func: ValidatedFunction<GenericQueryCtx<DataModel>, ArgsValidator, Output>,
  ): RegisteredQuery<Visibility, ObjectType<ArgsValidator>, Output>;

  <Output, Args extends ArgsArray = OneArgArray>(
    func: UnvalidatedFunction<GenericQueryCtx<DataModel>, Args, Output>,
  ): RegisteredQuery<Visibility, ArgsArrayToObject<Args>, Output>;
};

/**
 * Internal type helper used by Convex code generation.
 *
 * Used to give {@link actionGeneric} a type specific to your data model.
 * @public
 */
export type ActionBuilder<
  DataModel extends GenericDataModel,
  Visibility extends FunctionVisibility,
> = {
  <Output, ArgsValidator extends PropertyValidators>(
    func: ValidatedFunction<GenericActionCtx<DataModel>, ArgsValidator, Output>,
  ): RegisteredAction<Visibility, ObjectType<ArgsValidator>, Output>;

  <Output, Args extends ArgsArray = OneArgArray>(
    func: UnvalidatedFunction<GenericActionCtx<DataModel>, Args, Output>,
  ): RegisteredAction<Visibility, ArgsArrayToObject<Args>, Output>;
};

/**
 * Internal type helper used by Convex code generation.
 *
 * Used to give {@link httpActionGeneric} a type specific to your data model
 * and functions.
 * @public
 */
export type HttpActionBuilder = (
  func: (ctx: GenericActionCtx<any>, request: Request) => Promise<Response>,
) => PublicHttpAction;
