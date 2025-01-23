import { action, mutation, query } from "./_generated/server";

export const queryLogging = query(async () => {
  console.log("Important logged stuff");
});

export const queryLoggingAndThrowing = query(async () => {
  console.log("Important logged stuff");
  throw new Error("oopsie");
});

export const mutationLogging = mutation(async () => {
  console.log("Important logged stuff");
});

export const mutationLoggingAndThrowing = mutation(async () => {
  console.log("Important logged stuff");
  throw new Error("oopsie");
});

export const actionLogging = action(async () => {
  console.log("Important logged stuff");
});

export const actionLoggingAndThrowing = action(async () => {
  console.log("Important logged stuff");
  throw new Error("oopsie");
});
