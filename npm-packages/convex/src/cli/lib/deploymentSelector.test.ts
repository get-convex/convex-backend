import { describe, test, expect } from "vitest";
import { parseDeploymentSelector } from "./deploymentSelector.js";

describe("parseDeploymentSelector", () => {
  test('"dev"', () => {
    expect(parseDeploymentSelector("dev")).toEqual({
      kind: "inCurrentProject",
      selector: { kind: "dev" },
    });
  });

  test('"prod"', () => {
    expect(parseDeploymentSelector("prod")).toEqual({
      kind: "inCurrentProject",
      selector: { kind: "prod" },
    });
  });

  test('"local"', () => {
    expect(parseDeploymentSelector("local")).toEqual({
      kind: "local",
    });
  });

  test('"tall-forest-123"', () => {
    expect(parseDeploymentSelector("tall-forest-123")).toEqual({
      kind: "deploymentName",
      deploymentName: "tall-forest-123",
    });
  });

  test("reference", () => {
    expect(parseDeploymentSelector("dev/vercel")).toEqual({
      kind: "inCurrentProject",
      selector: { kind: "reference", reference: "dev/vercel" },
    });
  });

  test('"myproject:dev"', () => {
    expect(parseDeploymentSelector("myproject:dev")).toEqual({
      kind: "inProject",
      projectSlug: "myproject",
      selector: { kind: "dev" },
    });
  });

  test('"myproject:prod"', () => {
    expect(parseDeploymentSelector("myproject:prod")).toEqual({
      kind: "inProject",
      projectSlug: "myproject",
      selector: { kind: "prod" },
    });
  });

  test('"myproject:preview-name"', () => {
    expect(parseDeploymentSelector("myproject:preview-name")).toEqual({
      kind: "inProject",
      projectSlug: "myproject",
      selector: { kind: "reference", reference: "preview-name" },
    });
  });

  test('"myteam:myproject:dev"', () => {
    expect(parseDeploymentSelector("myteam:myproject:dev")).toEqual({
      kind: "inTeamProject",
      teamSlug: "myteam",
      projectSlug: "myproject",
      selector: { kind: "dev" },
    });
  });

  test('"myteam:myproject:prod"', () => {
    expect(parseDeploymentSelector("myteam:myproject:prod")).toEqual({
      kind: "inTeamProject",
      teamSlug: "myteam",
      projectSlug: "myproject",
      selector: { kind: "prod" },
    });
  });

  test('"myteam:myproject:preview-name"', () => {
    expect(parseDeploymentSelector("myteam:myproject:preview-name")).toEqual({
      kind: "inTeamProject",
      teamSlug: "myteam",
      projectSlug: "myproject",
      selector: { kind: "reference", reference: "preview-name" },
    });
  });
});
