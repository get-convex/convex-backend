import { httpAction } from "./_generated/server";

export const nop = null;

export const imported = httpAction(async (_ctx, _request: Request) => {
  return new Response("success");
});

export {};
