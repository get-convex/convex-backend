import { render, screen } from "@testing-library/react";
import { ConvexProvider } from "convex/react";
import { ProjectEnvVarConfig } from "hooks/api";
import {
  mockConvexReactClient,
  ConnectedDeploymentContext,
} from "dashboard-common";
import mockRouter from "next-router-mock";
import udfs from "udfs";
import { EnvironmentVariable } from "system-udfs/convex/_system/frontend/common";
import {
  DeploymentEnvironmentVariables,
  diffEnvironmentVariables,
} from "./DeploymentEnvironmentVariables";

jest.mock("api/roles", () => ({
  useHasProjectAdminPermissions: jest.fn(),
}));
jest.mock("api/profile", () => {});
jest.mock("api/backups", () => {});
jest.mock("api/deployments", () => ({
  useCurrentDeployment: jest.fn(),
}));
jest.mock("api/projects", () => ({
  useCurrentProject: jest.fn(),
}));
jest.mock("api/teams", () => ({
  useCurrentTeam: jest.fn(),
}));

const createEnvironmentVariable = (
  name: string,
  value: string,
): EnvironmentVariable => ({ name, value }) as EnvironmentVariable;

describe("DeploymentEnvironmentVariables", () => {
  describe("diffEnvironmentVariables", () => {
    it("exaclty same", () => {
      const projectLevelEnvVarDefaults: { configs: ProjectEnvVarConfig[] } = {
        configs: [
          {
            name: "name1",
            value: "value1",
            deploymentTypes: ["prod", "dev", "preview"],
          },
          {
            name: "name2",
            value: "value2",
            deploymentTypes: ["prod", "dev", "preview"],
          },
        ],
      };

      const deploymentEnvVariables = [
        createEnvironmentVariable("name1", "value1"),
        createEnvironmentVariable("name2", "value2"),
      ];

      const result = diffEnvironmentVariables(
        projectLevelEnvVarDefaults,
        deploymentEnvVariables,
        "dev",
      );
      expect(result).toEqual({ status: "same" });
    });

    it("deployment has more", () => {
      const projectLevelEnvVarDefaults: { configs: ProjectEnvVarConfig[] } = {
        configs: [
          {
            name: "name1",
            value: "value1",
            deploymentTypes: ["prod", "dev", "preview"],
          },
          {
            name: "name2",
            value: "value2",
            deploymentTypes: ["prod", "dev", "preview"],
          },
        ],
      };

      const deploymentEnvVariables = [
        createEnvironmentVariable("name1", "value1"),
        createEnvironmentVariable("name2", "value2"),
        createEnvironmentVariable("name3", "value3"),
      ];

      const result = diffEnvironmentVariables(
        projectLevelEnvVarDefaults,
        deploymentEnvVariables,
        "dev",
      );
      expect(result).toEqual({ status: "same" });
    });

    it("different", () => {
      const projectLevelEnvVarDefaults: { configs: ProjectEnvVarConfig[] } = {
        configs: [
          {
            name: "name1",
            value: "value1",
            deploymentTypes: ["prod", "dev", "preview"],
          },
          {
            name: "name2",
            value: "value2",
            deploymentTypes: ["prod", "dev", "preview"],
          },
        ],
      };

      const deploymentEnvVariables = [
        createEnvironmentVariable("name1", "value1"),
        createEnvironmentVariable("name2", "value3"),
      ];

      const result = diffEnvironmentVariables(
        projectLevelEnvVarDefaults,
        deploymentEnvVariables,
        "dev",
      );
      expect(result).toEqual({
        status: "different",
        projectEnvVariables: new Map([
          ["name1", "value1"],
          ["name2", "value2"],
        ]),
      });
    });

    it("respects deployment type", () => {
      const projectLevelEnvVarDefaults: { configs: ProjectEnvVarConfig[] } = {
        configs: [
          {
            name: "name1",
            value: "value1",
            deploymentTypes: ["prod", "dev", "preview"],
          },
          {
            name: "name2",
            value: "value2",
            deploymentTypes: ["preview"],
          },
        ],
      };

      const deploymentEnvVariables = [
        createEnvironmentVariable("name1", "value1"),
        createEnvironmentVariable("name2", "value3"),
      ];

      const result = diffEnvironmentVariables(
        projectLevelEnvVarDefaults,
        deploymentEnvVariables,
        "dev",
      );
      expect(result).toEqual({ status: "same" });
    });
  });
});

jest.mock("next/router", () => jest.requireActual("next-router-mock"));
describe("Prefilling env var name", () => {
  const mockClient = mockConvexReactClient().registerQueryFake(
    udfs.listEnvironmentVariables.default,
    () => [],
  );

  it("prefills one variable", async () => {
    mockRouter.setCurrentUrl("/some-url");
    mockRouter.query = { var: "MICHAL" };
    renderUI();
    const inputs = await screen.findAllByRole("textbox");
    expect(inputs).toHaveLength(2);
    expect(inputs[0]).toHaveValue("MICHAL");
    expect(inputs[1]).toHaveValue("");
  });

  it("prefills two variables", async () => {
    mockRouter.setCurrentUrl("/some-url");
    mockRouter.query = { var: ["MICHAL", "JAMES"] };
    renderUI();
    const inputs = await screen.findAllByRole("textbox");
    expect(inputs).toHaveLength(4);
    expect(inputs[0]).toHaveValue("MICHAL");
    expect(inputs[1]).toHaveValue("");
    expect(inputs[2]).toHaveValue("JAMES");
    expect(inputs[3]).toHaveValue("");
  });

  function renderUI() {
    render(
      <ConvexProvider client={mockClient}>
        <ConnectedDeploymentContext.Provider value={{ deployment: {} } as any}>
          <DeploymentEnvironmentVariables />
        </ConnectedDeploymentContext.Provider>
      </ConvexProvider>,
    );
  }
});
