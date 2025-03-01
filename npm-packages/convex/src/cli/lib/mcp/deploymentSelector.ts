import { DeploymentSelection, deploymentSelectionSchema } from "../api.js";

// Unfortunately, MCP clients don't seem to handle nested JSON objects very
// well (even though this is within spec). To work around this, encode the
// deployment selectors as an obfuscated string that the MCP client can
// opaquely pass around.
export function encodeDeploymentSelector(deployment: DeploymentSelection) {
  return `${deployment.kind}:${btoa(JSON.stringify(deployment))}`;
}

export function decodeDeploymentSelector(encoded: string) {
  const [_, encodedDeployment] = encoded.split(":");
  return deploymentSelectionSchema.parse(JSON.parse(atob(encodedDeployment)));
}
