// This file is for thick component clients and helpers that run

import { HttpRouter } from "convex/server";
import { GenericActionCtx, PublicHttpAction } from "convex/server";
import { httpAction } from "./ratelimiter/_generated/server.js";

// A version of httpAction that typechecks when used in other components.
type ClientHttpCtx = Omit<GenericActionCtx<any>, "vectorSearch"> & {
  vectorSearch: unknown;
};
type ClientExportedHttpCtx = Omit<GenericActionCtx<any>, "vectorSearch"> & {
  vectorSearch: any;
};
type OmitCallSignature<T> = T extends {
  (...args: any[]): any;
  [key: string]: any;
}
  ? { [K in keyof T as K extends `${string}` ? K : never]: T[K] }
  : T;
type ClientHttpAction = OmitCallSignature<PublicHttpAction> & {
  (ctx: ClientExportedHttpCtx, request: Request): Promise<Response>;
};
const clientHttpAction = httpAction as (
  func: (ctx: ClientHttpCtx, request: Request) => Promise<Response>,
) => ClientHttpAction;

export function add(a: number, b: number): number {
  return a + b;
}

// A client can export httpActions directly...
export const myHttpRoute = clientHttpAction(async (_) => {
  return new Response("OK");
});

// ...or a function that adds them to a router.
export function registerRoutes(exoticRouter: { isRouter: boolean }) {
  const router = exoticRouter as HttpRouter;
  router.route({
    path: "/ratelimiter/myHttpRoute",
    method: "GET",
    handler: myHttpRoute,
  });
}
