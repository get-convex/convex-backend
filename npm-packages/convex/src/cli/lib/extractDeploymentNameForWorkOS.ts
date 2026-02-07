/**
 * Extract deployment name from a Convex cloud URL for WorkOS provisioning.
 * Returns the deployment name if the URL matches the expected format, null otherwise.
 */
export function extractDeploymentNameForWorkOS(url: string): string | null {
  return (
    url.match(
      /^https:\/\/([a-z]+-[a-z]+-[0-9]+)\.(?:[^.]+\.)?convex\.cloud$/,
    )?.[1] ?? null
  );
}
