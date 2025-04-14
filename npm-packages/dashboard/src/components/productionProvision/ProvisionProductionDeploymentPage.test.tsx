import { render, cleanup } from "@testing-library/react";

import { PROVISION_PROD_PAGE_NAME } from "@common/lib/deploymentContext";
import { ProvisionProductionDeploymentPage } from "./ProvisionProductionDeploymentPage";

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
    query: { projectSlug: "myProject" },
    pathname: `/t/myTeam/myProject/${PROVISION_PROD_PAGE_NAME}`,
  }),
}));
jest.mock("@auth0/nextjs-auth0/client", () => ({
  useUser: () => ({
    user: {
      email: "test@convex.dev",
    },
  }),
}));

describe("ProvisionProductionDeploymentPage", () => {
  beforeEach(() => {
    cleanup();
    jest.clearAllMocks();
  });

  it("renders successfully", async () => {
    render(<ProvisionProductionDeploymentPage />);
  });
});
