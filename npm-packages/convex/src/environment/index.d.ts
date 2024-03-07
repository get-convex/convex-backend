/**
 * DEPRECATED: Convex now recommends using Node.js + browser types for all
 * Convex functions, for mutations, queries, and actions.
 * This file and the convex/environment entry point will be removed in a future
 * release.
 *
 * The Convex function environment.
 *
 * Query and mutation functions run in a limited environment within your deployment.
 * Here are the global APIs that are available in addition to the standard JavaScript ones.
 * To learn more, see [Query and Mutation Function Environment](https://docs.convex.dev/using/writing-convex-functions#environment).
 * @module
 */

interface Console {
  debug(...data: any[]): void;
  error(...data: any[]): void;
  info(...data: any[]): void;
  log(...data: any[]): void;
  warn(...data: any[]): void;
}

declare let console: Console;

/**
 * Environment variables can be accessed with `process.env.VAR_NAME`
 */
declare let process: { env: Record<string, string | undefined> };
