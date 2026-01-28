import { test, expect, describe } from "vitest";
import { partitionModulesByChanges, hash } from "./components.js";
import { Bundle, BundleHash } from "../../bundler/index.js";

describe("partitionModulesByChanges", () => {
  const createBundle = (
    path: string,
    source: string,
    environment: "node" | "isolate" = "isolate",
    sourceMap?: string,
  ): Bundle => ({
    path,
    source,
    environment,
    sourceMap,
  });

  const createBundleHash = (bundle: Bundle): BundleHash => ({
    path: bundle.path,
    hash: hash(bundle),
    environment: bundle.environment,
  });

  test("existing modules exist, but all modules are changed", () => {
    const oldFunctions = [
      createBundle("function1.js", "console.log('old1')"),
      createBundle("function2.js", "console.log('old2')"),
      createBundle("function3.js", "console.log('old3')"),
    ];
    const newFunctions = [
      createBundle("function1.js", "console.log('new1')"),
      createBundle("function2.js", "console.log('new2')"),
      createBundle("function3.js", "console.log('new3')"),
    ];

    const remoteHashesByPath = new Map(
      oldFunctions.map((f) => [f.path, createBundleHash(f)]),
    );

    const result = partitionModulesByChanges(newFunctions, remoteHashesByPath);

    expect(result.unchangedModuleHashes).toEqual([]);
    expect(result.changedModules).toEqual(newFunctions);
  });

  test("no existing modules", () => {
    const newFunctions = [
      createBundle("function1.js", "console.log('new1')"),
      createBundle("function2.js", "console.log('new2')"),
    ];

    const remoteHashesByPath = new Map<string, BundleHash>();

    const result = partitionModulesByChanges(newFunctions, remoteHashesByPath);

    expect(result.unchangedModuleHashes).toEqual([]);
    expect(result.changedModules).toEqual(newFunctions);
  });

  test("all files are the exact same", () => {
    const functions = [
      createBundle("function1.js", "console.log('same1')"),
      createBundle("function2.js", "console.log('same2')"),
      createBundle("function3.js", "console.log('same3')"),
    ];

    const remoteHashesByPath = new Map(
      functions.map((f) => [f.path, createBundleHash(f)]),
    );

    const result = partitionModulesByChanges(functions, remoteHashesByPath);

    expect(result.changedModules).toEqual([]);

    // Verify the unchanged module hashes have the expected structure
    expect(result.unchangedModuleHashes).toEqual([
      {
        path: "function1.js",
        environment: "isolate",
        sha256: hash(functions[0]),
      },
      {
        path: "function2.js",
        environment: "isolate",
        sha256: hash(functions[1]),
      },
      {
        path: "function3.js",
        environment: "isolate",
        sha256: hash(functions[2]),
      },
    ]);
  });

  test("some existing modules are the same and some are different", () => {
    const oldFunctions = [
      createBundle("function1.js", "console.log('same1')"),
      createBundle("function2.js", "console.log('old2')"),
      createBundle("function3.js", "console.log('same3')"),
    ];
    const newFunctions = [
      createBundle("function1.js", "console.log('same1')"),
      createBundle("function2.js", "console.log('new2')"),
      createBundle("function3.js", "console.log('same3')"),
    ];

    const remoteHashesByPath = new Map(
      oldFunctions.map((f) => [f.path, createBundleHash(f)]),
    );

    const result = partitionModulesByChanges(newFunctions, remoteHashesByPath);

    expect(result.unchangedModuleHashes.map((m) => m.path)).toEqual([
      "function1.js",
      "function3.js",
    ]);
    expect(result.changedModules.map((m) => m.path)).toEqual(["function2.js"]);
  });

  test("some existing modules are the same, but some are being deleted", () => {
    const oldFunctions = [
      createBundle("function1.js", "console.log('same1')"),
      createBundle("function2.js", "console.log('deleted')"),
      createBundle("function3.js", "console.log('same3')"),
    ];
    const newFunctions = [
      createBundle("function1.js", "console.log('same1')"),
      createBundle("function3.js", "console.log('same3')"),
    ];

    const remoteHashesByPath = new Map(
      oldFunctions.map((f) => [f.path, createBundleHash(f)]),
    );

    const result = partitionModulesByChanges(newFunctions, remoteHashesByPath);

    expect(result.changedModules).toEqual([]);

    expect(result.unchangedModuleHashes.map((m) => m.path)).toEqual([
      "function1.js",
      "function3.js",
    ]);
  });

  test("modules exist, but all modules are being deleted", () => {
    const oldFunctions = [
      createBundle("function1.js", "console.log('deleted1')"),
      createBundle("function2.js", "console.log('deleted2')"),
      createBundle("function3.js", "console.log('deleted3')"),
    ];
    const newFunctions: Bundle[] = [];

    const remoteHashesByPath = new Map(
      oldFunctions.map((f) => [f.path, createBundleHash(f)]),
    );

    const result = partitionModulesByChanges(newFunctions, remoteHashesByPath);

    expect(result.unchangedModuleHashes).toEqual([]);
    expect(result.changedModules).toEqual([]);
  });

  test("modules with different environments are considered changed", () => {
    const oldFunctions = [
      createBundle("function1.js", "console.log('same')", "node"),
    ];
    const newFunctions = [
      createBundle("function1.js", "console.log('same')", "isolate"),
    ];

    const remoteHashesByPath = new Map(
      oldFunctions.map((f) => [f.path, createBundleHash(f)]),
    );

    const result = partitionModulesByChanges(newFunctions, remoteHashesByPath);

    expect(result.unchangedModuleHashes).toEqual([]);
    expect(result.changedModules).toHaveLength(1);
    expect(result.changedModules[0].path).toBe("function1.js");
  });

  test("modules with different source maps are considered changed", () => {
    const oldFunctions = [
      createBundle("function1.js", "console.log('same')", "isolate", "old-map"),
    ];
    const newFunctions = [
      createBundle("function1.js", "console.log('same')", "isolate", "new-map"),
    ];

    const remoteHashesByPath = new Map(
      oldFunctions.map((f) => [f.path, createBundleHash(f)]),
    );

    const result = partitionModulesByChanges(newFunctions, remoteHashesByPath);

    expect(result.unchangedModuleHashes).toEqual([]);
    expect(result.changedModules).toHaveLength(1);
    expect(result.changedModules[0].path).toBe("function1.js");
  });

  test("new modules, changed modules, unchanged modules, and deleted modules", () => {
    const oldFunctions = [
      createBundle("unchanged.js", "console.log('unchanged')"),
      createBundle("changed.js", "console.log('old')"),
      createBundle("deleted.js", "console.log('deleted')"),
    ];
    const newFunctions = [
      createBundle("unchanged.js", "console.log('unchanged')"),
      createBundle("changed.js", "console.log('new')"),
      createBundle("new.js", "console.log('new')"),
    ];

    const remoteHashesByPath = new Map(
      oldFunctions.map((f) => [f.path, createBundleHash(f)]),
    );

    const result = partitionModulesByChanges(newFunctions, remoteHashesByPath);

    expect(result.unchangedModuleHashes.map((m) => m.path)).toEqual([
      "unchanged.js",
    ]);
    expect(result.changedModules.map((m) => m.path).sort()).toEqual([
      "changed.js",
      "new.js",
    ]);
  });
});
