import { Module } from "system-udfs/convex/_system/frontend/common";
import {
  modulesToModuleFunctions,
  displayNameToIdentifier,
} from "@common/lib/functions/FunctionsProvider";
import { findFunction } from "@common/features/functionRunner/lib/functionRunner";

describe("FunctionTester", () => {
  it("finds function with default export + folder with same name", () => {
    // regression test
    const modules: Map<string, Module> = new Map();
    modules.set("foo/bar.js", {
      functions: [
        {
          name: "baz",
          udfType: "Query",
          visibility: { kind: "public" },
          argsValidator: "",
        },
      ],
      sourcePackageId: "foo",
    });
    modules.set("foo.js", {
      functions: [
        {
          name: "default",
          udfType: "Mutation",
          visibility: { kind: "public" },
          argsValidator: "",
        },
      ],
      sourcePackageId: "foo",
    });
    const fileTree = modulesToModuleFunctions(new Map([[null, modules]]), []);
    const func = findFunction(fileTree, displayNameToIdentifier("foo"), null);
    expect(func).toBeDefined();
    expect(func?.identifier).toEqual("foo.js:default");
    expect(func?.udfType).toEqual("Mutation");
  });
});
