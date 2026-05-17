/**
 * Resolve the orchestrator base URL.
 *
 * - In production builds (and `next start`), this is configured at build time
 *   via `NEXT_PUBLIC_CONVEX_ORCHESTRATOR_URL`.
 * - In development we fall back to `http://localhost:8050` (the orchestrator's
 *   default `--provision-addr`).
 */
export function orchestratorUrl(): string {
  const env = process.env.NEXT_PUBLIC_CONVEX_ORCHESTRATOR_URL;
  if (env && env.length > 0) return env.replace(/\/$/, "");
  return "http://localhost:8050";
}
