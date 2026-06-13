import { FunctionType } from "./api.js";
import { FunctionVisibility } from "./registration.js";

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
 * Custom limits for a nested transaction. Each field specifies the absolute
 * maximum allowed for the nested function call. Values are capped at the
 * global transaction limits, so they can only lower limits, never raise them.
 *
 * @public
 */
export interface TransactionLimits {
  bytesRead?: number;
  bytesWritten?: number;
  databaseQueries?: number;
  documentsRead?: number;
  documentsWritten?: number;
  functionsScheduled?: number;
  scheduledFunctionArgsBytes?: number;
}

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
  /**
   * The ID of the scheduled function document (in `_scheduled_functions`) that
   * this execution belongs to, or `null` otherwise.
   *
   * This is set for the scheduled function itself and for any functions it
   * calls (e.g. a mutation invoked via `runMutation` by a scheduled action),
   * propagating the top-level scheduled function's ID down the call tree. It is
   * `null` when the function was not scheduled.
   *
   * @internal
   */
  scheduledFunctionId: string | null;
};

/**
 * Metadata about the deployment this function is running on.
 *
 * @public
 */
export type DeploymentMetadata = {
  /**
   * The deployment name, e.g. `"tall-tiger-123"` for cloud deployments,
   * `"local-my_team-my_project"` for local deployments, or
   * `"anonymous-*"` for anonymous deployments.
   */
  name: string;
  /**
   * The deployment region, e.g. `"aws-us-east-1"`.
   * `null` for local and self-hosted deployments.
   */
  region: string | null;
  /**
   * The deployment class, e.g. `"s16"`, `"s256"`, or `"d1024"`.
   */
  class: "s16" | "s256" | "d1024";
};

/**
 * Metadata about the HTTP request that triggered the current function execution.
 *
 * `ip` and `userAgent` are `null` when the function was not triggered by an
 * HTTP request (e.g. scheduled jobs or cron jobs).
 *
 * Functions called from within a function (i.e. using `runMutation` or
 * `runAction`) will have the same request metadata as the parent function.
 *
 * @public
 */
export type RequestMetadata = {
  ip: string | null;
  userAgent: string | null;
  requestId: string;
};

/**
 * Extra context available in Convex query functions.
 *
 * @public
 */
export interface QueryMeta {
  getFunctionMetadata(): Promise<FunctionMetadata>;
  getTransactionMetrics(): Promise<TransactionMetrics>;
  getDeploymentMetadata(): Promise<DeploymentMetadata>;
}

/**
 * Extra context available in Convex mutation functions.
 *
 * @public
 */
export interface MutationMeta extends QueryMeta {
  getRequestMetadata(): Promise<RequestMetadata>;
}

/**
 * Extra context available in Convex action functions.
 *
 * @public
 */
export interface ActionMeta {
  getFunctionMetadata(): Promise<FunctionMetadata>;
  getDeploymentMetadata(): Promise<DeploymentMetadata>;
  getRequestMetadata(): Promise<RequestMetadata>;
}
