import { describe, it, expect } from "vitest";
import { extractDeploymentNameForWorkOS } from "./extractDeploymentNameForWorkOS.js";

describe("extractDeploymentNameForWorkOS", () => {
  it("extracts deployment name from valid URL", () => {
    expect(
      extractDeploymentNameForWorkOS("https://happy-capybara-123.convex.cloud"),
    ).toEqual("happy-capybara-123");
  });

  it("extracts deployment name from EU region URL", () => {
    expect(
      extractDeploymentNameForWorkOS(
        "https://basic-whale-224.eu-west-1.convex.cloud",
      ),
    ).toEqual("basic-whale-224");
  });

  it("returns null for non-convex.cloud URLs", () => {
    expect(extractDeploymentNameForWorkOS("https://api.sync.t3.chat")).toEqual(
      null,
    );
  });

  it("returns null for convex.site", () => {
    expect(
      extractDeploymentNameForWorkOS("https://happy-capybara-123.convex.site"),
    ).toEqual(null);
  });
});
