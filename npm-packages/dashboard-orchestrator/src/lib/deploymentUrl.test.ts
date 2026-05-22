import { deploymentUrlForBrowser } from "./deploymentUrl";

describe("deploymentUrlForBrowser", () => {

  test("keeps http deployment URLs when the dashboard is served over http", () => {
    expect(deploymentUrlForBrowser("http://prod.localhost:9000", "http:")).toBe(
      "http://prod.localhost:9000",
    );
  });
});
