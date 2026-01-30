import "@testing-library/jest-dom";
import { render, screen, cleanup } from "@testing-library/react";
import mockRouter from "next-router-mock";
import { ProjectDetails } from "generatedApi";
import { useProjectById } from "api/projects";
import { ProjectCard } from "./ProjectCard";

jest.mock("api/projects", () => ({
  useProjectById: jest.fn(),
}));

jest.mock("next/router", () => jest.requireActual("next-router-mock"));

jest.mock("api/teams", () => ({
  useCurrentTeam: jest.fn().mockReturnValue({
    id: 1,
    slug: "test-team",
    name: "Test Team",
  }),
}));

jest.mock("api/roles", () => ({
  useHasProjectAdminPermissions: jest.fn().mockReturnValue(true),
}));

const mockUseProjectById = useProjectById as jest.MockedFunction<
  typeof useProjectById
>;

describe("ProjectCard", () => {
  const baseProject: ProjectDetails = {
    id: 1,
    name: "Test Project",
    slug: "test-project",
    isDemo: false,
    teamId: 1,
    createTime: Date.now(),
    prodDeploymentName: null,
    devDeploymentName: null,
  };

  beforeEach(() => {
    cleanup();
    jest.clearAllMocks();
    mockRouter.setCurrentUrl("/t/test-team/test-project");
  });

  it("renders project card with no deployments", () => {
    mockUseProjectById.mockReturnValue({
      project: {
        ...baseProject,
        prodDeploymentName: null,
        devDeploymentName: null,
      },
      isLoading: false,
      error: undefined,
    });

    render(<ProjectCard project={baseProject} />);

    expect(screen.getByText("Test Project")).toBeInTheDocument();
    expect(screen.getByText("test-project")).toBeInTheDocument();

    // Check card's main link - when no dev deployment, defaults to prod (provision page)
    const cardLink = screen.getByRole("link", { name: /Test Project/ });
    expect(cardLink).toHaveAttribute(
      "href",
      "/t/test-team/test-project/production",
    );

    // Check Production link - should point to provision page
    const productionLink = screen.getByRole("link", { name: "Production" });
    expect(productionLink).toHaveAttribute(
      "href",
      "/t/test-team/test-project/production",
    );

    // Check Development link - should point to provision page
    const developmentLink = screen.getByRole("link", { name: "Development" });
    expect(developmentLink).toHaveAttribute(
      "href",
      "/t/test-team/test-project/development",
    );
  });

  it("renders project card with dev deployment only", () => {
    mockUseProjectById.mockReturnValue({
      project: {
        ...baseProject,
        prodDeploymentName: null,
        devDeploymentName: "happy-capybara-123",
      },
      isLoading: false,
      error: undefined,
    });

    render(<ProjectCard project={baseProject} />);

    expect(screen.getByText("Test Project")).toBeInTheDocument();
    expect(screen.getByText("test-project")).toBeInTheDocument();

    // Check card's main link - when dev deployment exists, defaults to dev deployment
    const cardLink = screen.getByRole("link", { name: /Test Project/ });
    expect(cardLink).toHaveAttribute(
      "href",
      "/t/test-team/test-project/happy-capybara-123",
    );

    // Check Production link - should point to provision page
    const productionLink = screen.getByRole("link", { name: "Production" });
    expect(productionLink).toHaveAttribute(
      "href",
      "/t/test-team/test-project/production",
    );

    // Check Development link - should point to actual deployment
    const developmentLink = screen.getByRole("link", { name: "Development" });
    expect(developmentLink).toHaveAttribute(
      "href",
      "/t/test-team/test-project/happy-capybara-123",
    );
  });

  it("renders project card with prod deployment only", () => {
    mockUseProjectById.mockReturnValue({
      project: {
        ...baseProject,
        prodDeploymentName: "musical-otter-456",
        devDeploymentName: null,
      },
      isLoading: false,
      error: undefined,
    });

    render(<ProjectCard project={baseProject} />);

    expect(screen.getByText("Test Project")).toBeInTheDocument();
    expect(screen.getByText("test-project")).toBeInTheDocument();

    // Check card's main link - when no dev deployment, defaults to prod deployment
    const cardLink = screen.getByRole("link", { name: /Test Project/ });
    expect(cardLink).toHaveAttribute(
      "href",
      "/t/test-team/test-project/musical-otter-456",
    );

    // Check Production link - should point to actual deployment
    const productionLink = screen.getByRole("link", { name: "Production" });
    expect(productionLink).toHaveAttribute(
      "href",
      "/t/test-team/test-project/musical-otter-456",
    );

    // Check Development link - should point to provision page
    const developmentLink = screen.getByRole("link", { name: "Development" });
    expect(developmentLink).toHaveAttribute(
      "href",
      "/t/test-team/test-project/development",
    );
  });

  it("renders project card with both prod and dev deployments", () => {
    mockUseProjectById.mockReturnValue({
      project: {
        ...baseProject,
        prodDeploymentName: "musical-otter-456",
        devDeploymentName: "happy-capybara-123",
      },
      isLoading: false,
      error: undefined,
    });

    render(<ProjectCard project={baseProject} />);

    expect(screen.getByText("Test Project")).toBeInTheDocument();
    expect(screen.getByText("test-project")).toBeInTheDocument();

    // Check card's main link - when dev deployment exists, defaults to dev deployment
    const cardLink = screen.getByRole("link", { name: /Test Project/ });
    expect(cardLink).toHaveAttribute(
      "href",
      "/t/test-team/test-project/happy-capybara-123",
    );

    // Check Production link - should point to actual deployment
    const productionLink = screen.getByRole("link", { name: "Production" });
    expect(productionLink).toHaveAttribute(
      "href",
      "/t/test-team/test-project/musical-otter-456",
    );

    // Check Development link - should point to actual deployment
    const developmentLink = screen.getByRole("link", { name: "Development" });
    expect(developmentLink).toHaveAttribute(
      "href",
      "/t/test-team/test-project/happy-capybara-123",
    );
  });
});
