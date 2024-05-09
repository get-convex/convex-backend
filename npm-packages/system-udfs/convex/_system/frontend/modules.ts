import {
  CronSpec,
  Module,
  ResolvedSourcePos,
  UdfType,
  Visibility,
} from "./common";
import { queryPrivateSystem } from "../secretSystemTables";
import { v } from "convex/values";

export const getSourceCode = queryPrivateSystem({
  args: { path: v.string() },
  handler: async ({ db }, { path }): Promise<string | null> => {
    const module = await db
      .query("_modules")
      .withIndex("by_path", (q) => q.eq("path", path))
      .unique();
    if (!module) {
      return null;
    }
    const moduleVersion = await db
      .query("_module_versions")
      .withIndex("by_module_and_version", (q) => q.eq("module_id", module._id))
      .unique();
    if (!moduleVersion) {
      return null;
    }
    const analyzeResult = module.analyzeResult;
    if (!analyzeResult) {
      return null;
    }

    if (
      analyzeResult.sourceMapped &&
      analyzeResult.sourceMapped.sourceIndex !== null &&
      moduleVersion.sourceMap
    ) {
      const sourceIndex = Number(analyzeResult.sourceMapped.sourceIndex);
      try {
        const sourceMap = JSON.parse(moduleVersion.sourceMap);
        return sourceMap.sourcesContent[sourceIndex];
      } catch (e: any) {
        // Failed to load source map
      }
    }
    return null;
  },
});

export const list = queryPrivateSystem({
  args: {},
  handler: async ({ db }): Promise<[string, Module][]> => {
    const result: [string, Module][] = [];
    for await (const module of db.query("_modules")) {
      if (module.path.startsWith("_")) {
        continue;
      }
      const analyzeResult = module.analyzeResult;
      if (!analyzeResult) {
        // `Skipping ${module.path}`
        continue;
      }

      const functions =
        analyzeResult.sourceMapped?.functions.map(processFunction) ?? [];
      // Stuff HTTP routes into the functions (the format the dashboard expects).
      for (const route of analyzeResult.httpRoutes || []) {
        functions.push(processHttpRoute(route));
      }

      const cronSpecs = processCronSpecs(analyzeResult.cronSpecs);

      const moduleVersion = await db
        .query("_module_versions")
        .withIndex("by_module_and_version", (q) =>
          q.eq("module_id", module._id),
        )
        .unique();
      // The _modules entry exists so _module_versions most exist.
      if (!moduleVersion) {
        throw new Error(`Module version for ${module._id} not found`);
      }

      result.push([
        module.path,
        {
          functions,
          ...(cronSpecs !== null ? { cronSpecs } : {}),
          creationTime: moduleVersion._creationTime,
        } as Module,
      ]);
    }
    return result;
  },
});

function processCronSpecs(
  cronSpecs: null | undefined | Array<{ identifier: string; spec: CronSpec }>,
): Array<[string, CronSpec]> | null {
  if (cronSpecs === null || cronSpecs === undefined) {
    return null;
  }
  return cronSpecs.map((c) => [c.identifier, c.spec]);
}

function processHttpRoute(f: {
  route: {
    path: string;
    method: string;
  };
  pos?: ResolvedSourcePos;
  lineno?: bigint;
  args?: string;
}) {
  const lineno = f.pos?.start_lineno ?? f.lineno;
  return {
    name: `${f.route.method} ${f.route.path}`,
    lineno: lineno ? Number(lineno) : undefined,
    udfType: "HttpAction",
    visibility: { kind: "public" },
    argsValidator: f.args || '{ "type": "any" }',
  } as const;
}

function processFunction(f: {
  name: string;
  pos?: ResolvedSourcePos;
  lineno?: bigint;
  udfType: UdfType;
  visibility?: Visibility | null;
  args?: string;
}) {
  const lineno = f.pos?.start_lineno ?? f.lineno;
  return {
    ...f,
    lineno: lineno ? Number(lineno) : undefined,
    visibility: f.visibility ?? { kind: "public" },
    argsValidator: f.args || '{ "type": "any" }',
  };
}
