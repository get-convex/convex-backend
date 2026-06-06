import { describe, expect, test } from "vitest";
import { shouldForwardDeploymentCaptureMessage } from "./deploymentMessages";

describe("shouldForwardDeploymentCaptureMessage", () => {
  test("suppresses the upstream empty data page warning", () => {
    expect(
      shouldForwardDeploymentCaptureMessage(
        "Encountered unexpected state in data page: status: Exhausted, numRowsInTable: 0, numRowsRead: 0, isLoading: false",
        "warning",
      ),
    ).toBe(false);
  });

  test("forwards similar data page warnings with rows in the table", () => {
    expect(
      shouldForwardDeploymentCaptureMessage(
        "Encountered unexpected state in data page: status: Exhausted, numRowsInTable: 1, numRowsRead: 0, isLoading: false",
        "warning",
      ),
    ).toBe(true);
  });

  test("forwards unrelated warnings", () => {
    expect(
      shouldForwardDeploymentCaptureMessage(
        "Something else happened",
        "warning",
      ),
    ).toBe(true);
  });
});
