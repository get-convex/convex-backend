import { z } from "zod";
import {
  DeploymentSelectionWithinProject,
  deploymentSelectionWithinProjectSchema,
} from "../api.js";

// Unfortunately, MCP clients don't seem to handle nested JSON objects very
// well (even though this is within spec). To work around this, encode the
// deployment selectors as an obfuscated string that the MCP client can
// opaquely pass around.
export function encodeDeploymentSelector(
  projectDir: string,
  deployment: DeploymentSelectionWithinProject,
) {
  const payload = {
    projectDir,
    deployment,
  };
  return `${deployment.kind}:${btoa(JSON.stringify(payload))}`;
}

const payloadSchema = z.object({
  projectDir: z.string(),
  deployment: deploymentSelectionWithinProjectSchema,
});

export function decodeDeploymentSelector(encoded: string) {
  const [_, serializedPayload] = encoded.split(":");
  return payloadSchema.parse(JSON.parse(atob(serializedPayload)));
}
