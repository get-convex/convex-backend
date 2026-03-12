export type ParsedDeploymentSelector =
  | { kind: "defaultDev" }
  | { kind: "defaultProd" }
  | { kind: "deploymentName"; deploymentName: string }
  | { kind: "refInSameProject"; reference: string }
  | { kind: "refInOtherProject"; projectSlug: string; reference: string }
  | {
      kind: "refInOtherTeam";
      teamSlug: string;
      projectSlug: string;
      reference: string;
    };

/**
 * Parses the value of the `--deployment` CLI flag
 */
export function parseDeploymentSelector(
  selector: string,
): ParsedDeploymentSelector {
  if (selector === "dev") {
    return { kind: "defaultDev" };
  }
  if (selector === "prod") {
    return { kind: "defaultProd" };
  }
  if (/^[a-z]+-[a-z]+-[0-9]+$/.test(selector)) {
    return { kind: "deploymentName", deploymentName: selector };
  }
  const parts = selector.split(":");
  if (parts.length === 3) {
    return {
      kind: "refInOtherTeam",
      teamSlug: parts[0],
      projectSlug: parts[1],
      reference: parts[2],
    };
  }
  if (parts.length === 2) {
    return {
      kind: "refInOtherProject",
      projectSlug: parts[0],
      reference: parts[1],
    };
  }
  return { kind: "refInSameProject", reference: selector };
}
