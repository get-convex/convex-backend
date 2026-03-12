import { describe, test, expect } from "vitest";
import { parseDeploymentSelector } from "./deploymentSelector.js";

describe("parseDeploymentSelector", () => {
  test('"dev"', () => {
    expect(parseDeploymentSelector("dev")).toEqual({ kind: "defaultDev" });
  });

  test('"prod"', () => {
    expect(parseDeploymentSelector("prod")).toEqual({ kind: "defaultProd" });
  });

  test('"tall-forest-123"', () => {
    expect(parseDeploymentSelector("tall-forest-123")).toEqual({
      kind: "deploymentName",
      deploymentName: "tall-forest-123",
    });
  });

  test('"preview-name"', () => {
    expect(parseDeploymentSelector("preview-name")).toEqual({
      kind: "refInSameProject",
      reference: "preview-name",
    });
  });

  test('"myproject:prod"', () => {
    expect(parseDeploymentSelector("myproject:prod")).toEqual({
      kind: "refInOtherProject",
      projectSlug: "myproject",
      reference: "prod",
    });
  });

  test('"myteam:myproject:prod"', () => {
    expect(parseDeploymentSelector("myteam:myproject:prod")).toEqual({
      kind: "refInOtherTeam",
      teamSlug: "myteam",
      projectSlug: "myproject",
      reference: "prod",
    });
  });
});
