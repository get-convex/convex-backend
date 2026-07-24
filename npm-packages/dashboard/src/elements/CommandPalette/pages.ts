import type { PlatformDeploymentResponse, ProjectDetails } from "generatedApi";

// A layer in the palette's drill-in stack: a nested view the user has drilled
// into. The root is represented by an empty stack (no page); each drill pushes
// one of these.
export type PalettePage =
  | { type: "teams" }
  | { type: "projects" }
  | { type: "project"; project: ProjectDetails }
  // The deployments of a single project, without the project's own pages —
  // reached from the "Switch Deployment…" command.
  | { type: "deployments"; project: ProjectDetails }
  | {
      type: "deployment";
      deployment: PlatformDeploymentResponse;
      projectSlug?: string;
    }
  | { type: "components" }
  | { type: "theme" };

export function pageLabel(page: PalettePage): string {
  switch (page.type) {
    case "teams":
      return "Switch Team";
    case "projects":
      return "Switch Project";
    case "project":
      return page.project.name || page.project.slug;
    case "deployments":
      return "Switch Deployment";
    case "deployment":
      return "reference" in page.deployment
        ? page.deployment.reference
        : page.deployment.name;
    case "components":
      return "Switch Component";
    case "theme":
      return "Change Dashboard Theme";
    default: {
      page satisfies never;
      return "";
    }
  }
}
