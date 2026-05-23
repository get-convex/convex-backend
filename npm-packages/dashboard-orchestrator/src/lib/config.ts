// Runtime configuration for the orchestrator dashboard.
//
// Browser-facing URLs are read from server-side env at request time and
// injected into the page via `_document.tsx`'s
// `<script>window.__CONVEX_CONFIG__ = ...</script>` block, so the
// published image works on any host without a rebuild. Server-side
// callers (API routes, getServerSideProps) read process.env directly.

const DEFAULT_ORCHESTRATOR_URL = "http://localhost:8050";
const DEFAULT_ORCHESTRATOR_REGION_NAME = "Self-Hosted";

export type RuntimeConfig = {
  orchestratorUrl: string;
  orchestratorRegionName: string;
};

declare global {
  interface Window {
    __CONVEX_CONFIG__?: RuntimeConfig;
  }
}

/**
 * Server-side: build the config object that `_document.tsx` injects
 * into the page. Reads from process.env at request time, so changes to
 * the env take effect on the next request without a rebuild.
 */
export function getServerRuntimeConfig(): RuntimeConfig {
  return {
    orchestratorUrl: stripTrailingSlash(
      process.env.PUBLIC_ORCHESTRATOR_URL || DEFAULT_ORCHESTRATOR_URL,
    ),
    orchestratorRegionName:
      process.env.PUBLIC_ORCHESTRATOR_REGION_NAME ||
      DEFAULT_ORCHESTRATOR_REGION_NAME,
  };
}

/**
 * Returns the orchestrator base URL the browser should hit. Works in
 * both client and server contexts: on the server it reads process.env
 * directly; in the browser it reads the config object injected by
 * `_document.tsx`.
 */
export function orchestratorUrl(): string {
  if (typeof window !== "undefined" && window.__CONVEX_CONFIG__) {
    return window.__CONVEX_CONFIG__.orchestratorUrl;
  }
  return getServerRuntimeConfig().orchestratorUrl;
}

export function orchestratorRegionName(): string {
  if (typeof window !== "undefined" && window.__CONVEX_CONFIG__) {
    return (
      window.__CONVEX_CONFIG__.orchestratorRegionName ||
      DEFAULT_ORCHESTRATOR_REGION_NAME
    );
  }
  return getServerRuntimeConfig().orchestratorRegionName;
}

function stripTrailingSlash(s: string): string {
  return s.replace(/\/$/, "");
}
