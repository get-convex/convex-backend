import { getServerRuntimeConfig, orchestratorRegionName } from "./config";

const originalRegionName = process.env.PUBLIC_ORCHESTRATOR_REGION_NAME;

afterEach(() => {
  if (originalRegionName === undefined) {
    delete process.env.PUBLIC_ORCHESTRATOR_REGION_NAME;
  } else {
    process.env.PUBLIC_ORCHESTRATOR_REGION_NAME = originalRegionName;
  }
});

describe("runtime config", () => {
  test("defaults orchestrator deployments to the self-hosted region label", () => {
    delete process.env.PUBLIC_ORCHESTRATOR_REGION_NAME;

    expect(getServerRuntimeConfig().orchestratorRegionName).toBe("Self-Hosted");
  });

  test("uses the configured orchestrator deployment region label", () => {
    process.env.PUBLIC_ORCHESTRATOR_REGION_NAME = "Seoul Homelab";

    expect(getServerRuntimeConfig().orchestratorRegionName).toBe(
      "Seoul Homelab",
    );
    expect(orchestratorRegionName()).toBe("Seoul Homelab");
  });
});
