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
 * Extra context available in Convex query functions.
 *
 * @public
 */
export interface QueryMeta {
  getFunctionMetadata(): Promise<FunctionMetadata>;
  getTransactionMetrics(): Promise<TransactionMetrics>;
  /** @internal */
  getDeploymentMetadata(): Promise<DeploymentMetadata>;
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
  /** @internal */
  getDeploymentMetadata(): Promise<DeploymentMetadata>;
}
