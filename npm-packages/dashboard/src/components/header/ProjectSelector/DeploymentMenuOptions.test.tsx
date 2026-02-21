import React from "react";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DeploymentResponse, ProjectDetails, TeamResponse } from "generatedApi";
import { ContextMenu } from "@common/features/data/components/ContextMenu";
import { DeploymentMenuOptions } from "./DeploymentMenuOptions";

jest.mock("next/router", () => jest.requireActual("next-router-mock"));
jest.mock("api/profile", () => ({
  useProfile: jest.fn().mockReturnValue({ id: 1, name: "Test User" }),
}));
jest.mock("api/teams", () => ({
  useTeamMembers: jest.fn().mockReturnValue([]),
}));
jest.mock("api/deployments", () => ({
  useDefaultDevDeployment: jest.fn().mockReturnValue(undefined),
}));

const mockTeam: TeamResponse = {
  id: 1,
  name: "Test Team",
  creator: 1,
  slug: "test-team",
  suspended: false,
  referralCode: "CODE123",
};

const mockProject: ProjectDetails = {
  id: 1,
  name: "Test Project",
  slug: "test-project",
  teamId: 1,
  createTime: Date.now(),
  isDemo: false,
};

let deploymentId = 1;
function createDeployment(
  overrides: Partial<DeploymentResponse> & {
    name: string;
    deploymentType: DeploymentResponse["deploymentType"];
  },
): DeploymentResponse {
  return {
    id: deploymentId++,
    createTime: Date.now(),
    projectId: 1,
    kind: "cloud",
    region: "us-east-1",
    isDefault: false,
    ...overrides,
  } as DeploymentResponse;
}

describe("DeploymentMenuOptions", () => {
  const renderComponent = async (deployments: DeploymentResponse[]) => {
    await act(async () => {
      render(
        <ContextMenu target={{ x: 0, y: 0 }} onClose={jest.fn()}>
          <DeploymentMenuOptions
            team={mockTeam}
            project={mockProject}
            deployments={deployments}
          />
        </ContextMenu>,
      );
    });
  };

  describe("custom deployments", () => {
    test("when there are no custom deployments, 'Custom' doesn't appear", async () => {
      const deployments: DeploymentResponse[] = [
        createDeployment({ name: "prod-deployment", deploymentType: "prod" }),
        createDeployment({
          name: "dev-deployment",
          deploymentType: "dev",
          creator: 1,
        }),
      ];

      await renderComponent(deployments);

      expect(screen.queryByText("Custom Deployments")).not.toBeInTheDocument();
      expect(screen.queryByText(/custom/i)).not.toBeInTheDocument();
    });

    test("when there is at least one custom deployment, they appear under 'Custom Deployments' submenu", async () => {
      const deployments: DeploymentResponse[] = [
        createDeployment({ name: "prod-deployment", deploymentType: "prod" }),
        createDeployment({
          name: "custom-deployment-1",
          deploymentType: "custom",
        }),
        createDeployment({
          name: "custom-deployment-2",
          deploymentType: "custom",
        }),
      ];

      await renderComponent(deployments);

      // The submenu label should be visible
      expect(screen.getByText("Custom Deployments")).toBeInTheDocument();
      // The count should be shown
      expect(screen.getByText("2 deployments")).toBeInTheDocument();
    });
  });

  describe("prod deployments", () => {
    test("when there are no prod deployments, shows 'Select to create a Prod deployment'", async () => {
      const deployments: DeploymentResponse[] = [
        createDeployment({
          name: "dev-deployment",
          deploymentType: "dev",
          creator: 1,
        }),
      ];

      await renderComponent(deployments);

      expect(
        screen.getByText("Select to create a Prod deployment"),
      ).toBeInTheDocument();
    });

    test("when there is a single default prod deployment, it appears as a single option", async () => {
      const deployments: DeploymentResponse[] = [
        createDeployment({
          name: "my-prod-deployment",
          deploymentType: "prod",
          isDefault: true,
        }),
      ];

      await renderComponent(deployments);

      // Should show the Production identifier
      expect(screen.getByText("Production")).toBeInTheDocument();
      // Should show the deployment name
      expect(screen.getByText("my-prod-deployment")).toBeInTheDocument();
      // Should NOT show the "select to create" text
      expect(
        screen.queryByText("Select to create a Prod deployment"),
      ).not.toBeInTheDocument();
    });

    test("when there are multiple prod deployments, they appear in a submenu", async () => {
      const deployments: DeploymentResponse[] = [
        createDeployment({
          name: "prod-deployment-1",
          deploymentType: "prod",
          isDefault: true,
        }),
        createDeployment({
          name: "prod-deployment-2",
          deploymentType: "prod",
          isDefault: false,
        }),
      ];

      await renderComponent(deployments);

      // Should show a "Production" submenu with count
      expect(screen.getByText("2 deployments")).toBeInTheDocument();
    });

    test("when there are multiple prod deployments, the default one shows the keyboard shortcut and appears first", async () => {
      const user = userEvent.setup();
      const now = Date.now();
      const deployments: DeploymentResponse[] = [
        // Non-default created later (would normally sort first by createTime)
        createDeployment({
          name: "wandering-fish-513",
          deploymentType: "prod",
          isDefault: false,
          createTime: now + 1000,
          reference: "staging",
        }),
        // Default created earlier
        createDeployment({
          name: "knowing-antelope-914",
          deploymentType: "prod",
          isDefault: true,
          createTime: now,
          reference: "production",
        }),
      ];

      await renderComponent(deployments);

      // Open the Production submenu by hovering over it
      const productionSubmenu = screen.getByText("Production");
      await user.hover(productionSubmenu);

      // Wait for the submenu to open
      await waitFor(() => {
        expect(screen.getByText("Ctrl+Alt+1")).toBeInTheDocument();
      });

      // Verify the default deployment appears first by checking order of menu items
      // Each deployment name appears twice (as identifier and name in DeploymentOption)
      // so we look for the first occurrence of each
      const allText = screen.getAllByText(/(production|staging)/);
      // First two should be default
      expect(allText[0]).toHaveTextContent("production");
      // Next should be non-default
      expect(allText[1]).toHaveTextContent("staging");
    });

    test("when there is a single non-default prod deployment, it appears in a submenu", async () => {
      const deployments: DeploymentResponse[] = [
        createDeployment({
          name: "non-default-prod",
          deploymentType: "prod",
          isDefault: false,
        }),
      ];

      await renderComponent(deployments);

      // Should show a "Production" submenu with count
      expect(screen.getByText("1 deployment")).toBeInTheDocument();
    });
  });

  describe("other deployments (team member dev deployments)", () => {
    beforeEach(() => {
      // Mock team members for these tests
      const { useTeamMembers } = jest.requireMock("api/teams");
      useTeamMembers.mockReturnValue([
        { id: 2, name: "Alice", email: "alice@example.com" },
        { id: 3, name: "Bob", email: "bob@example.com" },
      ]);
    });

    afterEach(() => {
      const { useTeamMembers } = jest.requireMock("api/teams");
      useTeamMembers.mockReturnValue([]);
    });

    test("non-default dev deployments from other members appear in 'Other Deployments' submenu", async () => {
      const user = userEvent.setup();
      const deployments: DeploymentResponse[] = [
        createDeployment({
          name: "alice-non-default-dev",
          deploymentType: "dev",
          creator: 2,
          isDefault: false,
        }),
      ];

      await renderComponent(deployments);

      // Should show "Other Deployments" submenu with count
      expect(screen.getByText("Other Deployments")).toBeInTheDocument();
      expect(screen.getByText("1 deployment")).toBeInTheDocument();

      // Open the submenu
      const otherDeploymentsSubmenu = screen.getByText("Other Deployments");
      await user.hover(otherDeploymentsSubmenu);

      // Wait for the submenu to open and verify the deployment appears
      await waitFor(() => {
        expect(screen.getByText("alice-non-default-dev")).toBeInTheDocument();
      });
    });

    test("non-default dev deployments appear above default dev deployments from other members", async () => {
      const user = userEvent.setup();
      const now = Date.now();
      const deployments: DeploymentResponse[] = [
        // Alice's default dev deployment (created first)
        createDeployment({
          name: "alice-default-dev",
          deploymentType: "dev",
          creator: 2,
          isDefault: true,
          createTime: now,
        }),
        // Bob's non-default dev deployment (created later)
        createDeployment({
          name: "bob-non-default-dev",
          deploymentType: "dev",
          creator: 3,
          isDefault: false,
          createTime: now + 1000,
        }),
      ];

      await renderComponent(deployments);

      // Open the "Other Deployments" submenu
      const otherDeploymentsSubmenu = screen.getByText("Other Deployments");
      await user.hover(otherDeploymentsSubmenu);

      // Wait for the submenu to open
      await waitFor(() => {
        expect(screen.getByText("bob-non-default-dev")).toBeInTheDocument();
      });

      // Verify non-default deployment appears before default deployment
      const allDeploymentNames = screen.getAllByText(
        /alice-default-dev|bob-non-default-dev/,
      );
      expect(allDeploymentNames[0]).toHaveTextContent("bob-non-default-dev");
      expect(allDeploymentNames[1]).toHaveTextContent("alice-default-dev");
    });

    test("multiple non-default dev deployments appear above multiple default dev deployments", async () => {
      const user = userEvent.setup();
      const now = Date.now();
      const deployments: DeploymentResponse[] = [
        // Alice's default dev deployment
        createDeployment({
          name: "alice-default-dev",
          deploymentType: "dev",
          creator: 2,
          isDefault: true,
          createTime: now,
        }),
        // Bob's default dev deployment
        createDeployment({
          name: "bob-default-dev",
          deploymentType: "dev",
          creator: 3,
          isDefault: true,
          createTime: now + 100,
        }),
        // Alice's non-default dev deployment
        createDeployment({
          name: "alice-non-default-dev",
          deploymentType: "dev",
          creator: 2,
          isDefault: false,
          createTime: now + 200,
        }),
        // Bob's non-default dev deployment
        createDeployment({
          name: "bob-non-default-dev",
          deploymentType: "dev",
          creator: 3,
          isDefault: false,
          createTime: now + 300,
        }),
      ];

      await renderComponent(deployments);

      // Should show count for all 4 deployments
      expect(screen.getByText("4 deployments")).toBeInTheDocument();

      // Open the "Other Deployments" submenu
      const otherDeploymentsSubmenu = screen.getByText("Other Deployments");
      await user.hover(otherDeploymentsSubmenu);

      // Wait for the submenu to open
      await waitFor(() => {
        expect(screen.getByText("alice-non-default-dev")).toBeInTheDocument();
      });

      // Verify all non-default deployments appear before all default deployments
      const allDeploymentNames = screen.getAllByText(
        /alice-default-dev|bob-default-dev|alice-non-default-dev|bob-non-default-dev/,
      );

      // First two should be non-default (sorted by creator name within the group)
      expect(allDeploymentNames[0]).toHaveTextContent("alice-non-default-dev");
      expect(allDeploymentNames[1]).toHaveTextContent("bob-non-default-dev");
      // Last two should be default (sorted by creator name within the group)
      expect(allDeploymentNames[2]).toHaveTextContent("alice-default-dev");
      expect(allDeploymentNames[3]).toHaveTextContent("bob-default-dev");
    });
  });
});
