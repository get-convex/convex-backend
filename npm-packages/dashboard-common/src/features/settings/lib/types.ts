export type ProjectEnvVarConfig = {
  name: string;
  value: string;
  deploymentTypes: ("dev" | "preview" | "prod")[];
};
