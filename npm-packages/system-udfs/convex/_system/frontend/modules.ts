import {
  CronSpec,
  Module,
  ResolvedSourcePos,
  UdfType,
  Visibility,
} from "./common";
import { queryPrivateSystem } from "../secretSystemTables";

/**
 * Return all user defined modules + their functions.
 *
 * Note that this does not include system modules because they are not stored
 * in the `_modules` table.
 */
export const list = queryPrivateSystem({
  args: {},
  handler: async ({ db }): Promise<[string, Module][]> => {
    const result: [string, Module][] = [];
    for await (const module of db.query("_modules")) {
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

      result.push([
        module.path,
        {
          functions,
          sourcePackageId: module.sourcePackageId,
          ...(cronSpecs !== null ? { cronSpecs } : {}),
        },
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
