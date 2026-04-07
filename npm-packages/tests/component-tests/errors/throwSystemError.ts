import { action, query } from "./_generated/server";

declare const Convex: {
  syscall: (op: string, jsonArgs: string) => string;
};

export const fromQuery = query(async () => {
  Convex.syscall("throwSystemError", "{}");
});

export const fromAction = action(async () => {
  Convex.syscall("throwSystemError", "{}");
});
