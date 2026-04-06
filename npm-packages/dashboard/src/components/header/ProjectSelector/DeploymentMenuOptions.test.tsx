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
    deploymentClass: "s16",
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
        createDeployment({
          name: "happy-elephant-101",
          deploymentType: "prod",
        }),
        createDeployment({
          name: "lazy-panda-202",
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
        createDeployment({
          name: "happy-elephant-101",
          deploymentType: "prod",
        }),
        createDeployment({
          name: "bright-falcon-111",
          deploymentType: "custom",
        }),
        createDeployment({
          name: "swift-tiger-222",
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
          name: "lazy-panda-202",
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
          name: "calm-dolphin-789",
          deploymentType: "prod",
          isDefault: true,
        }),
      ];

      await renderComponent(deployments);

      // Should show the Production identifier
      expect(screen.getByText("Production")).toBeInTheDocument();
      // Should show the deployment name
      expect(screen.getByText("calm-dolphin-789")).toBeInTheDocument();
      // Should NOT show the "select to create" text
      expect(
        screen.queryByText("Select to create a Prod deployment"),
      ).not.toBeInTheDocument();
    });

    test("when there are multiple prod deployments, they appear in a submenu", async () => {
      const deployments: DeploymentResponse[] = [
        createDeployment({
          name: "happy-elephant-101",
          deploymentType: "prod",
          isDefault: true,
        }),
        createDeployment({
          name: "calm-dolphin-789",
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
          name: "quick-lion-987",
          deploymentType: "prod",
          isDefault: false,
          createTime: now + 1000,
          reference: "staging",
        }),
        // Default created earlier
        createDeployment({
          name: "gentle-bear-654",
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
          name: "quiet-badger-333",
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

    test("non-default dev deployments from other members appear in main menu when fewer than 10", async () => {
      const deployments: DeploymentResponse[] = [
        createDeployment({
          name: "fancy-rabbit-444",
          deploymentType: "dev",
          creator: 2,
          isDefault: false,
          reference: "wonderful-fish-321",
        }),
      ];

      await renderComponent(deployments);

      // Should appear directly in the main menu, not in Other Deployments
      // Non-default dev deployments show reference as the prominent identifier
      expect(screen.getByText("wonderful-fish-321")).toBeInTheDocument();
    });

    test("non-default team devs appear in main menu and defaults in Other Deployments", async () => {
      const user = userEvent.setup();
      const now = Date.now();
      const deployments: DeploymentResponse[] = [
        // Alice's default dev deployment (created first)
        createDeployment({
          name: "gentle-bear-654",
          deploymentType: "dev",
          creator: 2,
          isDefault: true,
          createTime: now,
        }),
        // Bob's non-default dev deployment (created later)
        createDeployment({
          name: "quick-lion-987",
          deploymentType: "dev",
          creator: 3,
          isDefault: false,
          createTime: now + 1000,
          reference: "bob-ref",
        }),
      ];

      await renderComponent(deployments);

      // Non-default should be in the main menu, shown by its reference
      expect(screen.getByText("bob-ref")).toBeInTheDocument();

      // Other Deployments should only contain the default dev
      expect(screen.getByText("1 deployment")).toBeInTheDocument();
      const otherDeploymentsSubmenu = screen.getByText("Other Deployments");
      await user.hover(otherDeploymentsSubmenu);

      await waitFor(() => {
        // Default dev in submenu is shown as "<MemberName>'s dev"
        expect(screen.getByText("Alice's dev")).toBeInTheDocument();
      });
    });

    test("when >= 10 non-default team devs, all go to Other Deployments", async () => {
      const user = userEvent.setup();
      const now = Date.now();
      const deployments: DeploymentResponse[] = [
        // Alice's default dev
        createDeployment({
          name: "gentle-bear-654",
          deploymentType: "dev",
          creator: 2,
          isDefault: true,
          createTime: now,
        }),
        // 10 non-default devs from Bob
        ...Array.from({ length: 10 }, (_, i) =>
          createDeployment({
            name: `brave-wolf-${100 + i}`,
            deploymentType: "dev",
            creator: 3,
            isDefault: false,
            createTime: now + i + 1,
            reference: `bob-ref-${i}`,
          }),
        ),
      ];

      await renderComponent(deployments);

      // Non-default devs should NOT be in the main menu (check by reference)
      expect(screen.queryByText("bob-ref-0")).not.toBeInTheDocument();

      // Other Deployments should contain all 11 (10 non-default + 1 default)
      expect(screen.getByText("11 deployments")).toBeInTheDocument();

      const otherDeploymentsSubmenu = screen.getByText("Other Deployments");
      await user.hover(otherDeploymentsSubmenu);

      await waitFor(() => {
        // Non-default devs are shown by their reference
        expect(screen.getByText("bob-ref-0")).toBeInTheDocument();
        // Default dev is shown as "<MemberName>'s dev"
        expect(screen.getByText("Alice's dev")).toBeInTheDocument();
      });
    });
  });
});
