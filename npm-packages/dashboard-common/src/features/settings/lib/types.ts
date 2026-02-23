export type ProjectEnvVarConfig = {
  name: string;
  value: string;
  deploymentTypes: readonly ("dev" | "preview" | "prod" | "custom")[];
};
