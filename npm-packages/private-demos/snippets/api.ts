import { FunctionReference, anyApi } from "convex/server";

export const api: PublicApiType = anyApi as unknown as PublicApiType;

export type PublicApiType = {
  messages: {
    send: FunctionReference<
      "mutation",
      "public",
      { author: string; body: string },
      null
    >;
  };
};
