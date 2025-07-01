import React from "react";
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { SettingsSidebar } from "./SettingsSidebar";

jest.mock("@common/lib/useNents", () => ({
  useNents: jest.fn().mockReturnValue({ nents: [] }),
}));

jest.mock("next/router", () => ({
  useRouter: jest.fn().mockReturnValue({
    query: {},
  }),
}));

describe("SettingsSidebar", () => {
  describe("cloud dashboard (dashboard.convex.dev)", () => {
    beforeEach(() => {
      render(
        <DeploymentInfoContext.Provider
          value={{
            ...mockDeploymentInfo,
            isSelfHosted: false,
            teamsURI: "/t/test-team",
            projectsURI: "/t/test-team/test-project",
            deploymentsURI: "/t/test-team/test-project/fine-marmot-266",
          }}
        >
          <SettingsSidebar selectedPage="url-and-deploy-key" />
        </DeploymentInfoContext.Provider>,
      );
    });

    test("First tab has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "URL & Deploy Key",
      });

      expect(link).toHaveAttribute(
        "href",
        "/t/test-team/test-project/fine-marmot-266/settings/",
      );
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });

    test("Standard tab has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Environment Variables",
      });

      expect(link).toHaveAttribute(
        "href",
        "/t/test-team/test-project/fine-marmot-266/settings/environment-variables",
      );
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });

    test("Backups link has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Backup & Restore",
      });

      expect(link).toHaveAttribute(
        "href",
        "/t/test-team/test-project/fine-marmot-266/settings/backups",
      );
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });

    test("Integrations link has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Integrations",
      });

      expect(link).toHaveAttribute(
        "href",
        "/t/test-team/test-project/fine-marmot-266/settings/integrations",
      );
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });

    test("Project Settings link has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Project Settings",
      });

      expect(link).toHaveAttribute(
        "href",
        "/t/test-team/test-project/settings",
      );
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });

    test("Project Usage link has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Project Usage",
      });

      expect(link).toHaveAttribute(
        "href",
        "/t/test-team/settings/usage?projectSlug=project",
      );
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });
  });

  describe("self-hosted dashboard with self-hosted deployment", () => {
    beforeEach(() => {
      render(
        <DeploymentInfoContext.Provider
          value={{
            ...mockDeploymentInfo,
            isSelfHosted: true,
            teamsURI: "",
            projectsURI: "",
            deploymentsURI: "",
          }}
        >
          <SettingsSidebar selectedPage="url-and-deploy-key" />
        </DeploymentInfoContext.Provider>,
      );
    });

    test("First tab has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "URL & Deploy Key",
      });

      expect(link).toHaveAttribute("href", "/settings/");
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });

    test("Standard tab has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Environment Variables",
      });

      expect(link).toHaveAttribute("href", "/settings/environment-variables");
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });

    test("Backups link is disabled in self-hosted deployment", async () => {
      const disabledLink = await screen.findByRole("button", {
        name: "Backup & Restore",
      });
      expect(disabledLink).toHaveAttribute("aria-disabled", "true");
    });

    test("Integrations tab has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Integrations",
      });
      expect(link).toHaveAttribute("href", "/settings/integrations");
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });

    test("Project Settings link is disabled in self-hosted deployment", async () => {
      const disabledLink = await screen.findByRole("button", {
        name: "Project Settings",
      });
      expect(disabledLink).toHaveAttribute("aria-disabled", "true");
    });

    test("Project Usage link is disabled in self-hosted deployment", async () => {
      const disabledLink = await screen.findByRole("button", {
        name: "Project Usage",
      });
      expect(disabledLink).toHaveAttribute("aria-disabled", "true");
    });
  });

  describe("self-hosted dashboard with cloud deployment", () => {
    beforeEach(() => {
      render(
        <DeploymentInfoContext.Provider
          value={{
            ...mockDeploymentInfo,
            isSelfHosted: true,
            teamsURI: "",
            projectsURI: "",
            deploymentsURI: "",
            ok: true,
            deploymentUrl: "https://fine-marmot-266.convex.cloud",
            adminKey: "test-admin-key",
          }}
        >
          <SettingsSidebar selectedPage="url-and-deploy-key" />
        </DeploymentInfoContext.Provider>,
      );
    });

    test("First tab has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "URL & Deploy Key",
      });

      expect(link).toHaveAttribute("href", "/settings/");
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });

    test("Standard tab has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Environment Variables",
      });

      expect(link).toHaveAttribute("href", "/settings/environment-variables");
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });

    test("Backups link has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Backup & Restore",
      });

      expect(link).toHaveAttribute(
        "href",
        "https://dashboard.convex.dev/d/fine-marmot-266/settings/backups",
      );
      expect(link).toHaveAttribute("target", "_blank");
      expect(link).not.toBeDisabled();
    });

    test("Integrations link has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Integrations",
      });

      expect(link).toHaveAttribute("href", "/settings/integrations");
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });

    test("Project Settings link has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Project Settings",
      });

      expect(link).toHaveAttribute(
        "href",
        "https://dashboard.convex.dev/dp/fine-marmot-266/settings",
      );
      expect(link).toHaveAttribute("target", "_blank");
      expect(link).not.toBeDisabled();
    });

    test("Project Usage link has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Project Usage",
      });

      expect(link).toHaveAttribute(
        "href",
        "https://dashboard.convex.dev/dp/fine-marmot-266/usage",
      );
      expect(link).toHaveAttribute("target", "_blank");
      expect(link).not.toBeDisabled();
    });
  });

  describe("paused team", () => {
    const pausedTeamContext = {
      ...mockDeploymentInfo,
      useTeamUsageState: jest.fn().mockReturnValue("Paused"),
    };

    beforeEach(() => {
      render(
        <DeploymentInfoContext.Provider value={pausedTeamContext}>
          <SettingsSidebar selectedPage="url-and-deploy-key" />
        </DeploymentInfoContext.Provider>,
      );
    });

    test("Pause Deployment link is locked when team is paused", async () => {
      const disabledLink = await screen.findByRole("button", {
        name: "Pause Deployment",
      });
      expect(disabledLink).toHaveAttribute("aria-disabled", "true");
    });
  });
});
