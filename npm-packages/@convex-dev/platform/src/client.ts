import createClient from "openapi-fetch";

import type { paths as ConvexPaths } from "./generatedApi.js";
import { version } from "./version.js";

export const productionProvisionHost = "https://api.convex.dev";
export const provisionHost = productionProvisionHost;
// This API spec is rooted here
const baseUrl = `${provisionHost}/api/dashboard`;

export type ConvexAuth = {
  kind: "accessToken";
  accessToken: string;
};

export function createConvexClient(accessToken: string) {
  const auth = {
    kind: "accessToken",
    accessToken,
  };

  const headers: Record<string, string> = {
    Authorization: `Bearer ${auth.accessToken}`,
    "Convex-Client": `convex-platform-${version}`,
  };
  console.log(headers);

  const client = createClient<ConvexPaths>({
    baseUrl,
    headers,
  });
  return client;
}
