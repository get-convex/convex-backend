import { type } from "arktype";

/**
 * Fetch the latest version of the convex package from the NPM registry
 */
export async function fetchLatestNpmVersion(): Promise<string> {
  const response = await fetch("https://registry.npmjs.org/convex/latest");
  if (!response.ok) {
    throw new Error(`Failed to fetch NPM data: ${response.status}`);
  }

  const NpmResponse = type({
    version: "string",
  });

  const out = NpmResponse(await response.json());
  if (out instanceof type.errors) {
    throw new Error("Invalid NPM response: " + out.summary);
  }

  return out.version;
}
