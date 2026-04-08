export type InProjectSelector =
  | { kind: "dev" }
  | { kind: "prod" }
  | { kind: "reference"; reference: string };

export type ParsedDeploymentSelector =
  | { kind: "deploymentName"; deploymentName: string }
  | { kind: "local" }
  | { kind: "inCurrentProject"; selector: InProjectSelector }
  | { kind: "inProject"; projectSlug: string; selector: InProjectSelector }
  | {
      kind: "inTeamProject";
      teamSlug: string;
      projectSlug: string;
      selector: InProjectSelector;
    };

function parseInProjectSelector(s: string): InProjectSelector {
  if (s === "dev") return { kind: "dev" };
  if (s === "prod") return { kind: "prod" };
  return { kind: "reference", reference: s };
}

/**
 * Parses the value of the `--deployment` CLI flag
 */
export function parseDeploymentSelector(
  selector: string,
): ParsedDeploymentSelector {
  if (selector === "local") return { kind: "local" };
  if (/^[a-z]+-[a-z]+-[0-9]+$/.test(selector)) {
    return { kind: "deploymentName", deploymentName: selector };
  }
  const parts = selector.split(":");
  if (parts.length === 3) {
    return {
      kind: "inTeamProject",
      teamSlug: parts[0],
      projectSlug: parts[1],
      selector: parseInProjectSelector(parts[2]),
    };
  }
  if (parts.length === 2) {
    return {
      kind: "inProject",
      projectSlug: parts[0],
      selector: parseInProjectSelector(parts[1]),
    };
  }
  return {
    kind: "inCurrentProject",
    selector: parseInProjectSelector(selector),
  };
}
