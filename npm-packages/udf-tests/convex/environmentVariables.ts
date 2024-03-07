import { action, query } from "./_generated/server";

const global = process.env.TEST_NAME;

export const getEnvironmentVariable = query(async () => {
  return process.env.TEST_NAME;
});

export const actionGetEnvironmentVariable = action(async () => {
  return process.env.TEST_NAME;
});

export const getGlobalEnvironmentVariable = query(async () => {
  return global;
});

export const actionGetGlobalEnvironmentVariable = action(async () => {
  return global;
});

export const log = query(async () => {
  console.log(process.env);
});

export const getCloudUrl = query(async () => {
  return process.env.CONVEX_CLOUD_URL;
});
export const getSiteUrl = query(async () => {
  return process.env.CONVEX_SITE_URL;
});
