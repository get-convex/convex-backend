import { action, mutation, query } from "./_generated/server";

export const queryLogging = query({
  args: {},
  handler: async () => {
    console.log("Important logged stuff");
  },
});

export const queryLoggingAndThrowing = query({
  args: {},
  handler: async () => {
    console.log("Important logged stuff");
    throw new Error("oopsie");
  },
});

export const mutationLogging = mutation({
  args: {},
  handler: async () => {
    console.log("Important logged stuff");
  },
});

export const mutationLoggingAndThrowing = mutation({
  args: {},
  handler: async () => {
    console.log("Important logged stuff");
    throw new Error("oopsie");
  },
});

export const actionLogging = action({
  args: {},
  handler: async () => {
    console.log("Important logged stuff");
  },
});

export const actionLoggingAndThrowing = action({
  args: {},
  handler: async () => {
    console.log("Important logged stuff");
    throw new Error("oopsie");
  },
});
