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
            useCurrentDeployment: () => ({
              id: 0,
              name: "fine-marmot-266",
              deploymentType: "prod",
              projectId: 0,
              kind: "cloud",
              previewIdentifier: null,
            }),
          }}
        >
          <SettingsSidebar selectedPage="general" />
        </DeploymentInfoContext.Provider>,
      );
    });

    test("First tab has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "General",
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
            ok: true,
            deploymentUrl: "https://my-selfhosted-deployment.example.com",
            adminKey: "test-admin-key",
            useCurrentDeployment: () => ({
              id: 0,
              name: "self-hosted",
              deploymentType: "prod",
              projectId: 0,
              kind: "cloud",
              previewIdentifier: null,
            }),
          }}
        >
          <SettingsSidebar selectedPage="general" />
        </DeploymentInfoContext.Provider>,
      );
    });

    test("First tab has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "General",
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

    test("Custom Domains link is disabled in self-hosted deployment", async () => {
      const disabledLink = await screen.findByRole("button", {
        name: "Custom Domains",
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
            useCurrentDeployment: () => ({
              id: 0,
              name: "fine-marmot-266",
              deploymentType: "prod",
              projectId: 0,
              kind: "cloud",
              previewIdentifier: null,
            }),
          }}
        >
          <SettingsSidebar selectedPage="general" />
        </DeploymentInfoContext.Provider>,
      );
    });

    test("First tab has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "General",
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

  describe("local deployment in cloud dashboard", () => {
    beforeEach(() => {
      render(
        <DeploymentInfoContext.Provider
          value={{
            ...mockDeploymentInfo,
            isSelfHosted: false,
            teamsURI: "/t/test-team",
            projectsURI: "/t/test-team/test-project",
            deploymentsURI: "/t/test-team/test-project/fine-marmot-266",
            useCurrentDeployment: () => ({
              id: 0,
              name: "local",
              deploymentType: "dev",
              projectId: 0,
              kind: "local",
              previewIdentifier: null,
            }),
          }}
        >
          <SettingsSidebar selectedPage="general" />
        </DeploymentInfoContext.Provider>,
      );
    });

    test("General has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "General",
      });

      expect(link).toHaveAttribute(
        "href",
        "/t/test-team/test-project/fine-marmot-266/settings/",
      );
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });

    test("Environment Variables has correct URL and is enabled", async () => {
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

    test("Custom Domains is disabled in local deployment", async () => {
      const disabledLink = await screen.findByRole("button", {
        name: "Custom Domains",
      });
      expect(disabledLink).toHaveAttribute("aria-disabled", "true");
    });

    test("Backups is disabled in local deployment", async () => {
      const disabledLink = await screen.findByRole("button", {
        name: "Backup & Restore",
      });
      expect(disabledLink).toHaveAttribute("aria-disabled", "true");
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
  });

  describe("cloud dashboard - custom domains tests", () => {
    beforeEach(() => {
      render(
        <DeploymentInfoContext.Provider
          value={{
            ...mockDeploymentInfo,
            isSelfHosted: false,
            teamsURI: "/t/test-team",
            projectsURI: "/t/test-team/test-project",
            deploymentsURI: "/t/test-team/test-project/fine-marmot-266",
            useCurrentDeployment: () => ({
              id: 0,
              name: "fine-marmot-266",
              deploymentType: "prod",
              projectId: 0,
              kind: "cloud",
              previewIdentifier: null,
            }),
          }}
        >
          <SettingsSidebar selectedPage="custom-domains" />
        </DeploymentInfoContext.Provider>,
      );
    });

    test("Custom Domains link has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Custom Domains",
      });

      expect(link).toHaveAttribute(
        "href",
        "/t/test-team/test-project/fine-marmot-266/settings/custom-domains",
      );
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });
  });

  describe("self-hosted dashboard with cloud deployment - custom domains", () => {
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
            useCurrentDeployment: () => ({
              id: 0,
              name: "fine-marmot-266",
              deploymentType: "prod",
              projectId: 0,
              kind: "cloud",
              previewIdentifier: null,
            }),
          }}
        >
          <SettingsSidebar selectedPage="custom-domains" />
        </DeploymentInfoContext.Provider>,
      );
    });

    test("Custom Domains link has correct URL and is enabled", async () => {
      const link = await screen.findByRole("link", {
        name: "Custom Domains",
      });

      expect(link).toHaveAttribute("href", "/settings/custom-domains");
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toBeDisabled();
    });
  });
});
