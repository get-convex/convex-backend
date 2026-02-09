import { ProjectEnvVarConfig } from "@common/features/settings/lib/types";
import { validateProjectEnvVarUniqueness } from "./DefaultEnvironmentVariables";

describe("validateProjectEnvVarUniqueness", () => {
  const createVariable = (
    name: string,
    formKey: string,
    deploymentTypes: string[],
  ): { name: string; formKey: string; envVar: ProjectEnvVarConfig } => ({
    name,
    formKey,
    envVar: {
      name,
      value: `value_for_${name}`,
      deploymentTypes:
        deploymentTypes as ProjectEnvVarConfig["deploymentTypes"],
    },
  });

  it("allows same name with non-overlapping deployment types", () => {
    const variables = [
      createVariable("SECRET_KEY", "newVars[0]", ["dev"]),
      createVariable("SECRET_KEY", "newVars[1]", ["prod"]),
    ];
    const errors = validateProjectEnvVarUniqueness(variables);
    expect(errors).toEqual({});
  });

  it("rejects same name with overlapping deployment types", () => {
    const variables = [
      createVariable("SECRET_KEY", "newVars[0]", ["dev", "prod"]),
      createVariable("SECRET_KEY", "newVars[1]", ["prod", "preview"]),
    ];
    const errors = validateProjectEnvVarUniqueness(variables);
    expect(errors["newVars[0].deploymentTypes"]).toContain("Production");
    expect(errors["newVars[1].deploymentTypes"]).toContain("Production");
  });

  it("doesn’t show errors for empty env var names", () => {
    const variables = [
      createVariable("", "newVars[0]", ["dev", "prod", "preview"]),
      createVariable("", "newVars[1]", ["dev", "prod", "preview"]),
    ];
    const errors = validateProjectEnvVarUniqueness(variables);
    expect(errors).toEqual({});
  });

  it("allows different names with any deployment types", () => {
    const variables = [
      createVariable("KEY_A", "newVars[0]", ["dev", "prod"]),
      createVariable("KEY_B", "newVars[1]", ["dev", "prod"]),
    ];
    const errors = validateProjectEnvVarUniqueness(variables);
    expect(errors).toEqual({});
  });

  it("handles three-way conflicts correctly", () => {
    const variables = [
      createVariable("KEY", "newVars[0]", ["dev"]),
      createVariable("KEY", "newVars[1]", ["dev", "preview"]),
      createVariable("KEY", "newVars[2]", ["preview"]),
    ];
    const errors = validateProjectEnvVarUniqueness(variables);
    // First and second conflict on "dev"
    expect(errors["newVars[0].deploymentTypes"]).toBeDefined();
    expect(errors["newVars[1].deploymentTypes"]).toBeDefined();
    // Second and third conflict on "preview"
    expect(errors["newVars[2].deploymentTypes"]).toBeDefined();
  });

  it("allows three variables with completely non-overlapping deployment types", () => {
    const variables = [
      createVariable("KEY", "newVars[0]", ["dev"]),
      createVariable("KEY", "newVars[1]", ["preview"]),
      createVariable("KEY", "newVars[2]", ["prod"]),
    ];
    const errors = validateProjectEnvVarUniqueness(variables);
    expect(errors).toEqual({});
  });

  it("handles mixed edited and new variables", () => {
    const variables = [
      createVariable("SECRET", "editedVars[0].newEnvVar", ["dev"]),
      createVariable("SECRET", "newVars[0]", ["dev"]), // conflicts with edited
    ];
    const errors = validateProjectEnvVarUniqueness(variables);
    expect(errors["editedVars[0].newEnvVar.deploymentTypes"]).toBeDefined();
    expect(errors["newVars[0].deploymentTypes"]).toBeDefined();
  });

  it("handles single variable without errors", () => {
    const variables = [
      createVariable("ONLY_VAR", "newVars[0]", ["dev", "preview", "prod"]),
    ];
    const errors = validateProjectEnvVarUniqueness(variables);
    expect(errors).toEqual({});
  });

  it("handles empty array", () => {
    const errors = validateProjectEnvVarUniqueness([]);
    expect(errors).toEqual({});
  });

  it("handles custom deployment type in overlap detection", () => {
    const variables = [
      createVariable("KEY", "newVars[0]", ["custom"]),
      createVariable("KEY", "newVars[1]", ["custom"]),
    ];
    const errors = validateProjectEnvVarUniqueness(variables);
    expect(errors["newVars[0].deploymentTypes"]).toContain("Custom");
    expect(errors["newVars[1].deploymentTypes"]).toContain("Custom");
  });

  it("handles multiple overlapping types in error message", () => {
    const variables = [
      createVariable("KEY", "newVars[0]", ["dev", "preview"]),
      createVariable("KEY", "newVars[1]", ["dev", "preview"]),
    ];
    const errors = validateProjectEnvVarUniqueness(variables);
    expect(errors["newVars[0].deploymentTypes"]).toContain("Development");
    expect(errors["newVars[0].deploymentTypes"]).toContain("Preview");
  });

  it("rejects variables with empty deployment types array", () => {
    const variables = [createVariable("KEY", "newVars[0]", [])];
    const errors = validateProjectEnvVarUniqueness(variables);
    expect(errors["newVars[0].deploymentTypes"]).toBe(
      "At least one deployment type must be selected",
    );
  });

  it("rejects multiple variables with empty deployment types", () => {
    const variables = [
      createVariable("KEY_A", "newVars[0]", []),
      createVariable("KEY_B", "newVars[1]", []),
    ];
    const errors = validateProjectEnvVarUniqueness(variables);
    expect(errors["newVars[0].deploymentTypes"]).toBe(
      "At least one deployment type must be selected",
    );
    expect(errors["newVars[1].deploymentTypes"]).toBe(
      "At least one deployment type must be selected",
    );
  });
});
