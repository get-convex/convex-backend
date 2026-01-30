import { render, cleanup } from "@testing-library/react";

import { ProvisionDeploymentPage } from "./ProvisionDeploymentPage";

jest.mock("api/profile", () => {});
jest.mock("api/projects", () => ({
  useCurrentProject: jest.fn(),
}));
jest.mock("api/deployments", () => ({
  useCurrentDeployment: jest.fn(),
}));
jest.mock("api/teams", () => ({
  useCurrentTeam: jest.fn(),
}));
jest.mock("next/router", () => ({
  useRouter: () => ({
    query: { project: "myProject" },
    replace: jest.fn(),
  }),
}));

describe("ProvisionDeploymentPage", () => {
  beforeEach(() => {
    cleanup();
    jest.clearAllMocks();
  });

  it("renders successfully for production", async () => {
    const { container } = render(
      <ProvisionDeploymentPage deploymentType="prod" />,
    );
    expect(container.querySelector("h1")).toBeInTheDocument();
  });

  it("renders successfully for development", async () => {
    const { container } = render(
      <ProvisionDeploymentPage deploymentType="dev" />,
    );
    expect(container.querySelector("h1")).toBeInTheDocument();
  });
});
