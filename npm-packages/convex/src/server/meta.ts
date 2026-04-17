import { FunctionType } from "./api.js";
import { FunctionVisibility } from "./registration.js";
import { Value } from "../values/value.js";

/**
 * Used and remaining amounts for a single transaction limit.
 *
 * @public
 */
export type TransactionMetric = {
  used: number;
  remaining: number;
};

/**
 * The remaining headroom for a transaction before hitting limits.
 *
 * See https://docs.convex.dev/production/state/limits
 *
 * @public
 */
export type TransactionMetrics = {
  bytesRead: TransactionMetric;
  bytesWritten: TransactionMetric;
  databaseQueries: TransactionMetric;
  documentsRead: TransactionMetric;
  documentsWritten: TransactionMetric;
  functionsScheduled: TransactionMetric;
  scheduledFunctionArgsBytes: TransactionMetric;
};

/**
 * Metadata about the currently executing Convex function.
 *
 * @public
 */
export type FunctionMetadata = {
  /**
   * The name of the function, in the format `"path/to/module:functionName"`
   */
  name: string;
  /**
   * The path of the component this function belongs to.
   * This is an empty string `""` for the app.
   */
  componentPath: string;
  /** Whether it's a query, mutation, or action. */
  type: FunctionType;
  /** Whether the function is public or internal. */
  visibility: FunctionVisibility;
};

/**
 * Invocation metadata propagated alongside a Convex function invocation.
 *
 * Values follow the same serialization rules as function arguments and return
 * values.
 *
 * @public
 */
export type InvocationMetadata = Record<string, Value>;

/**
 * Runtime invocation context for the currently executing function.
 *
 * @public
 */
export type InvocationContext = {
  requestId: string;
  executionId: string;
  isRoot: boolean;
  parentScheduledJob?: string | null;
  parentScheduledJobComponentId?: string | null;
  metadata: InvocationMetadata | null;
};

/**
 * Optional call-site metadata for nested or one-shot function calls.
 *
 * Nested calls inherit parent invocation metadata by default. Passing
 * `metadata` overrides top-level keys for that call.
 *
 * @public
 */
export type InvocationOptions = {
  metadata?: InvocationMetadata;
};

/**
 * Extra context available in Convex query functions.
 *
 * @public
 */
export interface QueryMeta {
  getFunctionMetadata(): Promise<FunctionMetadata>;
  getTransactionMetrics(): Promise<TransactionMetrics>;
  /** Read the invocation context for the current function call. */
  getInvocationContext(): Promise<InvocationContext>;
}

/**
 * Extra context available in Convex mutation functions.
 *
 * @public
 */
export interface MutationMeta extends QueryMeta {}

/**
 * Extra context available in Convex action functions.
 *
 * @public
 */
export interface ActionMeta {
  getFunctionMetadata(): Promise<FunctionMetadata>;
  /** Read the invocation context for the current function call. */
  getInvocationContext(): Promise<InvocationContext>;
}
