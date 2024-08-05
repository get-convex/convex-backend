import { query, action, componentArg } from "./_generated/server";

export const hello = action(async (ctx) => {
  const name = componentArg(ctx, "name");
  console.log(`hi from ${name}`);
  return name;
});

export const url = action(async (ctx) => {
  return componentArg(ctx, "url");
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
