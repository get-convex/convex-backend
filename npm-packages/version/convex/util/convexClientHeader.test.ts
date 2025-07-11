import { describe, it, expect } from "vitest";
import { extractVersionFromHeader } from "./convexClientHeader";

describe("extractVersionFromHeader", () => {
  it("extracts basic semver version from npm-cli header", () => {
    const header = "npm-cli-1.2.3";
    expect(extractVersionFromHeader(header)).toBe("1.2.3");
  });

  it("extracts pre-release version with hyphen", () => {
    const header = "npm-cli-0.0.0-alpha-1";
    expect(extractVersionFromHeader(header)).toBe("0.0.0-alpha-1");
  });

  it("returns null for other clients", () => {
    const header = "fivetran-export-1.2.3";
    expect(extractVersionFromHeader(header)).toBeNull();
  });

  it("returns null for empty string", () => {
    const header = "";
    expect(extractVersionFromHeader(header)).toBeNull();
  });

  it("returns null for npm-cli without version", () => {
    const header = "npm-cli-";
    expect(extractVersionFromHeader(header)).toBeNull();
  });
});
