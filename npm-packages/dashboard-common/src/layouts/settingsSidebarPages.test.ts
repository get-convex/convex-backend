import { getAllowedDeploymentSettingsPages } from "./settingsSidebarPages";

describe("getAllowedDeploymentSettingsPages", () => {
  test("keeps the Components tab visible while component metadata is loading", () => {
    expect(
      getAllowedDeploymentSettingsPages({
        nents: undefined,
        showAdminKeys: true,
      }),
    ).toContain("components");
  });

  test("hides the Components tab when loaded component metadata is empty", () => {
    expect(
      getAllowedDeploymentSettingsPages({
        nents: [],
        showAdminKeys: true,
      }),
    ).not.toContain("components");
  });

  test("hides the Admin Keys tab when the deployment backend does not own admin keys", () => {
    expect(
      getAllowedDeploymentSettingsPages({
        nents: [{ name: "_App" }],
        showAdminKeys: false,
      }),
    ).not.toContain("admin-keys");
  });
});
