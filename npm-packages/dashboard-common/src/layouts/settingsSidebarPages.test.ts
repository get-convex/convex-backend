import { getAllowedDeploymentSettingsPages } from "./settingsSidebarPages";

describe("getAllowedDeploymentSettingsPages", () => {
  test("keeps the Components tab visible while component metadata is loading", () => {
    expect(
      getAllowedDeploymentSettingsPages({
        nents: undefined,
        showAdminKeys: true,
        usageLimitsEnabled: true,
      }),
    ).toContain("components");
  });

  test("hides the Components tab when loaded component metadata is empty", () => {
    expect(
      getAllowedDeploymentSettingsPages({
        nents: [],
        showAdminKeys: true,
        usageLimitsEnabled: true,
      }),
    ).not.toContain("components");
  });

  test("hides the Admin Keys tab when the deployment backend does not own admin keys", () => {
    expect(
      getAllowedDeploymentSettingsPages({
        nents: [{ name: "_App" }],
        showAdminKeys: false,
        usageLimitsEnabled: true,
      }),
    ).not.toContain("admin-keys");
  });

  test("hides the Usage Limits tab when the feature flag is off", () => {
    expect(
      getAllowedDeploymentSettingsPages({
        nents: [{ name: "_App" }],
        showAdminKeys: true,
        usageLimitsEnabled: false,
      }),
    ).not.toContain("usage-limits");
  });

  test("shows the Usage Limits tab when the feature flag is on", () => {
    expect(
      getAllowedDeploymentSettingsPages({
        nents: [{ name: "_App" }],
        showAdminKeys: true,
        usageLimitsEnabled: true,
      }),
    ).toContain("usage-limits");
  });
});
