import { Context } from "./context";
import { checkConvexVersionFromPackageJson } from "./load";

describe("version check", () => {
  const testContext = {
    spinner: undefined,
    crash: async (args) => {
      throw new Error(args.printedMessage ?? "no message");
    },
    addWarning: () => {
      throw new Error("not implemented");
    },
    incrementChanges: () => {
      throw new Error("not implemented");
    },
    printResults: () => {
      throw new Error("not implemented");
    },
  } satisfies Context;

  function packageJsonForVersion(version: string) {
    return JSON.stringify({
      dependencies: {
        convex: version,
      },
    });
  }

  it("should succeed on newer versions", async () => {
    await checkConvexVersionFromPackageJson(
      testContext,
      "test",
      packageJsonForVersion("1.26.0"),
      ">=1.26.0",
    );
  });

  it("should fail on older versions", async () => {
    await expect(
      checkConvexVersionFromPackageJson(
        testContext,
        "test",
        packageJsonForVersion("1.25.0"),
        ">=1.26.0",
      ),
    ).rejects.toThrow();
  });

  it("should fail if the convex version is a range containing older versions", async () => {
    await expect(
      checkConvexVersionFromPackageJson(
        testContext,
        "test",
        packageJsonForVersion(">=1.23.0 <1.28.0"),
        ">=1.26.0",
      ),
    ).rejects.toThrow();
  });
});
