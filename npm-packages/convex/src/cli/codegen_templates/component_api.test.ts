import { test, expect } from "vitest";
import type {
  AnalyzedFunction,
  AnalyzedModule,
} from "../lib/deployApi/modules.js";
import type { CanonicalizedModulePath } from "../lib/deployApi/paths.js";

const mockCtx = {
  crash: async (error: any) => {
    throw new Error(error.printedMessage);
  },
} as any;

function createMockFunction(name: string): AnalyzedFunction {
  return {
    name,
    udfType: "Query",
    visibility: { kind: "public" as const },
    args: "{}",
    returns: "any",
  } as AnalyzedFunction;
}

function createMockModule(functions: AnalyzedFunction[]): AnalyzedModule {
  return {
    functions,
  } as AnalyzedModule;
}

import { buildApiTree } from "./component_api.js";

test("should deduplicate single-function files matching filename", async () => {
  const functions: Record<CanonicalizedModulePath, AnalyzedModule> = {
    // Single-function files matching filename (should deduplicate)
    "blog/post/getComments.ts": createMockModule([
      createMockFunction("getComments"),
    ]),
    "admin/users/permissions/checkAccess.ts": createMockModule([
      createMockFunction("checkAccess"),
    ]),
    // Multi-function file (should NOT deduplicate)
    "blog/post/mutations.ts": createMockModule([
      createMockFunction("createPost"),
      createMockFunction("updatePost"),
    ]),
  } as Record<CanonicalizedModulePath, AnalyzedModule>;

  const tree = (await buildApiTree(mockCtx, functions, {
    kind: "public",
  })) as any;

  // Verify deduplication: single-function files with matching names
  expect(tree.blog?.branch?.post?.branch?.getComments?.leaf?.name).toBe(
    "getComments",
  );
  expect(
    tree.admin?.branch?.users?.branch?.permissions?.branch?.checkAccess?.leaf
      ?.name,
  ).toBe("checkAccess");

  // Verify no deduplication: multi-function files keep filename
  expect(
    tree.blog?.branch?.post?.branch?.mutations?.branch?.createPost?.leaf?.name,
  ).toBe("createPost");
  expect(
    tree.blog?.branch?.post?.branch?.mutations?.branch?.updatePost?.leaf?.name,
  ).toBe("updatePost");
});

test("should maintain backward compatibility with existing patterns", async () => {
  const functions: Record<CanonicalizedModulePath, AnalyzedModule> = {
    // Multi-function files - existing pattern should work unchanged
    "api/users.ts": createMockModule([
      createMockFunction("getUserById"),
      createMockFunction("listUsers"),
    ]),
    "blog/queries.ts": createMockModule([
      createMockFunction("getPosts"),
      createMockFunction("getPostById"),
    ]),
  } as Record<CanonicalizedModulePath, AnalyzedModule>;

  const tree = (await buildApiTree(mockCtx, functions, {
    kind: "public",
  })) as any;

  // All should keep their filenames
  expect(tree.api?.branch?.users?.branch?.getUserById?.leaf?.name).toBe(
    "getUserById",
  );
  expect(tree.api?.branch?.users?.branch?.listUsers?.leaf?.name).toBe(
    "listUsers",
  );
  expect(tree.blog?.branch?.queries?.branch?.getPosts?.leaf?.name).toBe(
    "getPosts",
  );
  expect(tree.blog?.branch?.queries?.branch?.getPostById?.leaf?.name).toBe(
    "getPostById",
  );
});
