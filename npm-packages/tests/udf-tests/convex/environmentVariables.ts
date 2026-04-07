import { action, query } from "./_generated/server";

const global = process.env.TEST_NAME;

export const getEnvironmentVariable = query({
  args: {},
  handler: async () => {
    return process.env.TEST_NAME;
  },
});

export const getOtherEnvironmentVariable = query({
  args: {},
  handler: async () => {
    return process.env.TEST_NAME_2;
  },
});

export const actionGetEnvironmentVariable = action({
  args: {},
  handler: async () => {
    return process.env.TEST_NAME;
  },
});

export const getGlobalEnvironmentVariable = query({
  args: {},
  handler: async () => {
    return global;
  },
});

export const actionGetGlobalEnvironmentVariable = action({
  args: {},
  handler: async () => {
    return global;
  },
});

export const log = query({
  args: {},
  handler: async () => {
    console.log(process.env);
  },
});

export const getCloudUrl = query({
  args: {},
  handler: async () => {
    return process.env.CONVEX_CLOUD_URL;
  },
});
export const getSiteUrl = query({
  args: {},
  handler: async () => {
    return process.env.CONVEX_SITE_URL;
  },
});
