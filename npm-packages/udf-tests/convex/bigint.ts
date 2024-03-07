import { mutation, query, action } from "./_generated/server";

export const intQuery = query(async () => {
  return 1n;
});

export const intMutation = mutation(async () => {
  return 1n;
});

export const intAction = action(async () => {
  return 1n;
});
