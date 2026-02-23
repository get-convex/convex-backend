import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ConvexProvider } from "convex/react";
import mockRouter from "next-router-mock";
import udfs from "@common/udfs";
import { EnvironmentVariable } from "system-udfs/convex/_system/frontend/common";
import {
  DeploymentEnvironmentVariables,
  diffEnvironmentVariables,
} from "@common/features/settings/components/DeploymentEnvironmentVariables";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import {
  ConnectedDeployment,
  ConnectedDeploymentContext,
  DeploymentInfoContext,
} from "@common/lib/deploymentContext";
import { ProjectEnvVarConfig } from "@common/features/settings/lib/types";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";

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

    it("different value", () => {
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
      // Only name2 differs; name1 matches and should not be included
      expect(result).toEqual({
        status: "different",
        projectEnvVariables: new Map([["name2", "value2"]]),
      });
    });

    it("missing from deployment", () => {
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
        // name2 is absent
      ];

      const result = diffEnvironmentVariables(
        projectLevelEnvVarDefaults,
        deploymentEnvVariables,
        "dev",
      );
      // Only name2 is missing; name1 matches
      expect(result).toEqual({
        status: "different",
        projectEnvVariables: new Map([["name2", "value2"]]),
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

// @ts-expect-error -- mock deployment value
const mockConnectedDeployment: ConnectedDeployment = {};

describe("Use project defaults button", () => {
  const deploymentEnvVars = [
    createEnvironmentVariable("VAR_UNCHANGED", "same_value"),
    createEnvironmentVariable("VAR_MODIFIED", "old_value"),
  ];

  const mockClient = mockConvexReactClient().registerQueryFake(
    udfs.listEnvironmentVariables.default,
    () => deploymentEnvVars,
  );

  const mockDeploymentInfoWithProjectVars = {
    ...mockDeploymentInfo,
    useProjectEnvironmentVariables: () => ({
      configs: [
        {
          name: "VAR_UNCHANGED",
          value: "same_value",
          deploymentTypes: ["prod"] as const,
        },
        {
          name: "VAR_MODIFIED",
          value: "new_value",
          deploymentTypes: ["prod"] as const,
        },
        {
          name: "VAR_NEW",
          value: "new_var_value",
          deploymentTypes: ["prod"] as const,
        },
      ],
    }),
  };

  it("only adds rows for missing or different env vars", async () => {
    const user = userEvent.setup();
    mockRouter.setCurrentUrl("/some-url");
    mockRouter.query = {};
    render(
      <DeploymentInfoContext.Provider value={mockDeploymentInfoWithProjectVars}>
        <ConvexProvider client={mockClient}>
          <ConnectedDeploymentContext.Provider
            value={{
              deployment: mockConnectedDeployment,
              isDisconnected: false,
            }}
          >
            <DeploymentEnvironmentVariables />
          </ConnectedDeploymentContext.Provider>
        </ConvexProvider>
      </DeploymentInfoContext.Provider>,
    );

    const button = await screen.findByRole("button", {
      name: "Use project defaults",
    });
    await user.click(button);

    // VAR_UNCHANGED matches → not added. VAR_MODIFIED and VAR_NEW → added.
    const inputs = await screen.findAllByRole("textbox");
    expect(inputs[0]).toHaveValue("VAR_MODIFIED");
    expect(inputs[1]).toHaveValue("new_value");
    expect(inputs[2]).toHaveValue("VAR_NEW");
    expect(inputs[3]).toHaveValue("new_var_value");
  });
});

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
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <ConvexProvider client={mockClient}>
          <ConnectedDeploymentContext.Provider
            value={{ deployment: {} } as any}
          >
            <DeploymentEnvironmentVariables />
          </ConnectedDeploymentContext.Provider>
        </ConvexProvider>
        ,
      </DeploymentInfoContext.Provider>,
    );
  }
});
