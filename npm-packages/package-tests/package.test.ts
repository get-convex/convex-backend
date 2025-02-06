/**
 * Sometimes copies of the convex end up try work together:
 * - in a monorepo, developers might try to compbine helpers or
 *   validators from multiple packages and these might, for weird
 *   pnpm peer dependency reasons or ordinary version skew reasons,
 *   be from different copies of Convex.
 * - in libraries that list convex as a dependency instead of a
 *   peerDependency (not advised, but a defensible choice) two versions
 *   of the convex package may be in play.
 * - when one package imports CommonJS types and another imports ESM,
 *   TypeScript won't consider these equivalent and they will also
 *   be different at runtime.
 *
 * Regardless of change in APIs, some types are unassignable across
 * multiple copies of the convex package because they contain nominal
 * types (caused by private class properties or recursive types
 * like our FilterBuilder) and some values will not compare equal,
 * like an error not being an instanceof ConvexError in the thick client.
 *
 * These tests check that types currently assignable remain assignable
 * and test that workarounds continue to work.
 *
 * If we implement helpers for cross-package compat in the future this
 * would be a good place to test them.
 */

import { test, expect, describe } from "vitest";

import { version } from "convex";
import { version as version1dot16 } from "convex1dot16";

import * as valuesMain from "convex/values";
import { Validator as ValidatorMain } from "convex/values";
import * as values1dot16 from "convex1dot16/values";
import { v as v1dot16 } from "convex1dot16/values";

import * as serverMain from "convex/server";
import * as server1dot16 from "convex1dot16/server";

test("all old installations work", async () => {
  expect(version).toMatch(/1\.\d+\.\d+/);
  // 1.16 was the first version with components support
  expect(version1dot16).equal("1.16.2");
});

// TODO as these the same?
type OmitCallSignature<T> = T extends {
  (...args: any[]): any;
  [key: string]: any;
}
  ? { [K in keyof T as K extends `${string}` ? K : never]: T[K] }
  : T;

type RemoveCallSignature<T> = {
  // eslint-disable-next-line @typescript-eslint/ban-types
  [K in keyof T as T[K] extends Function ? K : never]: T[K];
} & {
  // eslint-disable-next-line @typescript-eslint/ban-types
  [K in keyof T as T[K] extends Function ? never : K]: T[K];
};

describe("Assignability", () => {
  /** We've seen customers do this so it's nice for it to keep working. */
  test("Validators", () => {
    // Old validators can be assigned to the current Validator type.
    const _a: ValidatorMain<any, any> = v1dot16.string();
  });

  /** We have not recommended these workarounds to developers. */
  test("Function wrappers", () => {
    // Main function wrapers *cannot* be assigned to old function wrappers.
    // This isn't an incompatible changes thing, these types just aren't assignable
    // to the same types in another package.

    // @ts-expect-error Known issue: mutation builders are not assignable across packages
    const _b: server1dot16.MutationBuilder<any, any> =
      serverMain.mutationGeneric;

    // Workaround: use a looser type in the older library

    // A looser type for mutation because Convex function wrappers
    // are not assignable across convex packages.
    type PublicMutationWrapperGeneric = (
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      ...args: any[]
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    ) => RemoveCallSignature<
      server1dot16.RegisteredMutation<"public", any, any>
    >;

    const _: PublicMutationWrapperGeneric = serverMain.mutationGeneric;
  });

  /** This is known and it's a bummer. Maybe we'll add a util to identify
   * a ConvexError in the future. */
  test("ConvexError", () => {
    const eMain = new valuesMain.ConvexError("asdf");
    expect(eMain instanceof valuesMain.ConvexError).toBeTruthy();

    // Known: at runtime instanceof checks don't work across packages.
    expect(eMain instanceof values1dot16.ConvexError).toBeFalsy();
  });

  test("HttpActions", () => {
    const myHttpAction = server1dot16.httpActionGeneric((_ctx, _r: Request) =>
      Promise.resolve(new Response("asdf")),
    );
    // @ts-expect-error Known issue: vector search has a recursive type in it.
    const _httpAction: serverMain.PublicHttpAction = myHttpAction;

    // workaround: remove vector search
    type ClientHttpCtx = Omit<
      server1dot16.GenericActionCtx<any>,
      "vectorSearch"
    > & {
      vectorSearch: unknown;
    };
    type ClientExportedHttpCtx = Omit<
      server1dot16.GenericActionCtx<any>,
      "vectorSearch"
    > & {
      vectorSearch: any;
    };
    type ClientHttpAction = OmitCallSignature<server1dot16.PublicHttpAction> & {
      (ctx: ClientExportedHttpCtx, request: Request): Promise<Response>;
    };
    const clientHttpAction = server1dot16.httpActionGeneric as (
      func: (ctx: ClientHttpCtx, request: Request) => Promise<Response>,
    ) => ClientHttpAction;

    // A client can export httpActions directly
    const myHttpActionVectorSearchRemoved = clientHttpAction(async (_) => {
      return new Response("OK");
    });
    const _httpAction2: serverMain.PublicHttpAction =
      myHttpActionVectorSearchRemoved;
  });
});
