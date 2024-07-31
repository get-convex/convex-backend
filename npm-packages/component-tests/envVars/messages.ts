import { query, action, componentArgs } from "./_generated/server";

export const hello = action(async () => {
  console.log(`hi from ${componentArgs.name}`);
  return componentArgs.name;
});

export const url = action(async () => {
  return componentArgs.url;
});

export const envVarQuery = query(async () => {
  return process.env.NAME;
});

export const systemEnvVarQuery = query(async () => {
  return process.env.CONVEX_CLOUD_URL;
});

export const envVarAction = action(async () => {
  return process.env.NAME;
});

export const systemEnvVarAction = action(async () => {
  return process.env.CONVEX_CLOUD_URL;
});
