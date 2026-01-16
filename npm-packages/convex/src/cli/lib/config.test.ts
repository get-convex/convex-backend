import { vi, test, expect, beforeEach, MockInstance, beforeAll } from "vitest";
import {
  parseProjectConfig,
  ProjectConfig,
  writeProjectConfig,
  readProjectConfig,
  resetUnknownKeyWarnings,
} from "./config.js";
import { Context, oneoffContext } from "../../bundler/context.js";
import { logFailure } from "../../bundler/log.js";
import { stripVTControlCharacters } from "util";

let ctx: Context;
let stderrSpy: MockInstance;

beforeAll(async () => {
  stderrSpy = vi.spyOn(process.stderr, "write").mockImplementation(() => true);
});

beforeEach(async () => {
  const originalContext = await oneoffContext({
    url: undefined,
    adminKey: undefined,
    envFile: undefined,
  });
  ctx = {
    ...originalContext,
    crash: (args: { printedMessage: string | null }) => {
      if (args.printedMessage !== null) {
        logFailure(args.printedMessage);
      }
      throw new Error();
    },
  };
  stderrSpy.mockClear();
  resetUnknownKeyWarnings(); // Reset warning state between tests
});

const assertParses = async (
  inp: Record<string, any>,
  expected?: ProjectConfig,
) => {
  const result = await parseProjectConfig(ctx, inp);
  expect(result).toEqual(expected ?? inp);
};

const assertParseError = async (inp: any, err: string) => {
  stderrSpy.mockClear();
  await expect(parseProjectConfig(ctx, inp)).rejects.toThrow();
  const calledWith = stderrSpy.mock.calls as string[][];
  expect(stripVTControlCharacters(calledWith[0][0])).toEqual(err);
};

test("parseProjectConfig basic valid configs", async () => {
  await assertParses(
    {
      functions: "functions/",
    },
    {
      functions: "functions/",
      // default values (note that node version *has no default*
      node: { externalPackages: [] },
      generateCommonJSApi: false,
      codegen: { staticApi: false, staticDataModel: false },
    },
  );

  await assertParses(
    {
      functions: "functions/",

      // unknown property
      futureFeature: 123,

      // deprecated
      team: "team",
      project: "proj",
      prodUrl: "prodUrl",
      authInfo: [
        {
          applicationID: "hello",
          domain: "world",
        },
      ],
    },
    {
      functions: "functions/",
      // default values
      node: { externalPackages: [] },
      generateCommonJSApi: false,
      codegen: { staticApi: false, staticDataModel: false },

      // unknown properties are preserved
      ...({ futureFeature: 123 } as any),

      // deprecated
      team: "team",
      project: "proj",
      prodUrl: "prodUrl",
      authInfo: [
        {
          applicationID: "hello",
          domain: "world",
        },
      ],
    },
  );
});

test("parseProjectConfig - node defaults", async () => {
  // No node field -> gets defaulted
  await assertParses(
    {},
    {
      functions: "convex/",
      node: { externalPackages: [] },
      generateCommonJSApi: false,
      codegen: { staticApi: false, staticDataModel: false },
    },
  );

  // node exists but externalPackages missing -> gets defaulted
  await assertParses(
    { node: { extraField: 123 } },
    {
      functions: "convex/",
      node: { externalPackages: [], ...{ extraField: 123 } },
      generateCommonJSApi: false,
      codegen: { staticApi: false, staticDataModel: false },
    },
  );

  // node with nodeVersion but no externalPackages
  await assertParses(
    { node: { nodeVersion: "18", extraField: 123 } },
    {
      functions: "convex/",
      node: { externalPackages: [], nodeVersion: "18", ...{ extraField: 123 } },
      generateCommonJSApi: false,
      codegen: { staticApi: false, staticDataModel: false },
    },
  );
});

test("parseProjectConfig - node validation errors", async () => {
  await assertParseError(
    { node: { externalPackages: "not-an-array" } },
    "✖ `node.externalPackages` in `convex.json`: Expected array, received string\n",
  );

  await assertParseError(
    { node: { externalPackages: [123] } },
    "✖ `node.externalPackages.0` in `convex.json`: Expected string, received number\n",
  );

  await assertParseError(
    { node: { nodeVersion: 18 } },
    "✖ `node.nodeVersion` in `convex.json`: Expected string, received number\n",
  );
});

test("parseProjectConfig - codegen fields", async () => {
  // fileType with valid values
  await assertParses(
    { codegen: { fileType: "ts" } },
    {
      functions: "convex/",
      node: { externalPackages: [] },
      generateCommonJSApi: false,
      codegen: { staticApi: false, staticDataModel: false, fileType: "ts" },
    },
  );

  await assertParses(
    { codegen: { fileType: "js/dts" } },
    {
      functions: "convex/",
      node: { externalPackages: [] },
      generateCommonJSApi: false,
      codegen: { staticApi: false, staticDataModel: false, fileType: "js/dts" },
    },
  );

  // legacyComponentApi
  await assertParses(
    { codegen: { legacyComponentApi: false } },
    {
      functions: "convex/",
      node: { externalPackages: [] },
      generateCommonJSApi: false,
      codegen: {
        staticApi: false,
        staticDataModel: false,
        legacyComponentApi: false,
      },
    },
  );

  await assertParses(
    { codegen: { legacyComponentApi: true } },
    {
      functions: "convex/",
      node: { externalPackages: [] },
      generateCommonJSApi: false,
      codegen: {
        staticApi: false,
        staticDataModel: false,
        legacyComponentApi: true,
      },
    },
  );
});

test("parseProjectConfig - codegen validation errors", async () => {
  // Invalid fileType value
  await assertParseError(
    { codegen: { fileType: "invalid" } },
    "✖ `codegen.fileType` in `convex.json`: Invalid enum value. Expected 'ts' | 'js/dts', received 'invalid'\n",
  );

  // Invalid legacyComponentApi type
  await assertParseError(
    { codegen: { legacyComponentApi: "yes" } },
    "✖ `codegen.legacyComponentApi` in `convex.json`: Expected boolean, received string\n",
  );

  // Cross-field validation: generateCommonJSApi: true with fileType: "ts" should fail
  await assertParseError(
    { generateCommonJSApi: true, codegen: { fileType: "ts" } },
    '✖ `generateCommonJSApi` in `convex.json`: Cannot use `generateCommonJSApi: true` with `codegen.fileType: "ts"`. CommonJS modules require JavaScript generation. Either set `codegen.fileType: "js/dts"` or remove `generateCommonJSApi`.\n',
  );
});

test("parseProjectConfig - top-level validation", async () => {
  await assertParseError(
    "not-an-object",
    "✖ Expected `convex.json` to contain an object\n",
  );
  await assertParseError(
    123,
    "✖ Expected `convex.json` to contain an object\n",
  );
  await assertParseError(
    null,
    "✖ Expected `convex.json` to contain an object\n",
  );
  await assertParseError(
    [],
    "✖ Expected `convex.json` to contain an object\n",
  );
});

test("writeProjectConfig - creates functions directory", async () => {
  let mkdirCalled = false;
  let mkdirPath = "";
  const testCtx = {
    ...ctx,
    fs: {
      ...ctx.fs,
      exists: () => false,
      mkdir: (path: string) => {
        mkdirCalled = true;
        mkdirPath = path;
      },
    },
  };

  const config: ProjectConfig = {
    functions: "my-functions/",
    node: { externalPackages: [] },
    generateCommonJSApi: false,
    codegen: { staticApi: false, staticDataModel: false },
  };

  await writeProjectConfig(testCtx, config);
  expect(mkdirCalled).toBe(true);
  expect(mkdirPath).toBe("my-functions/");
});

test("writeProjectConfig - does not write to convex.json", async () => {
  let writeUtf8FileCalled = false;
  const testCtx = {
    ...ctx,
    fs: {
      ...ctx.fs,
      exists: () => false,
      writeUtf8File: () => {
        writeUtf8FileCalled = true;
      },
      mkdir: () => {},
    },
  };

  // Even with non-default config, should NOT write to convex.json
  const config: ProjectConfig = {
    functions: "my-functions/",
    node: { externalPackages: ["axios"] },
    generateCommonJSApi: true,
    codegen: { staticApi: true, staticDataModel: true },
  };

  await writeProjectConfig(testCtx, config);
  expect(writeUtf8FileCalled).toBe(false);
});

test("readProjectConfig - returns defaults when file doesn't exist", async () => {
  const testCtx = {
    ...ctx,
    fs: {
      ...ctx.fs,
      exists: () => false,
      readUtf8File: (path: string) => {
        // Mock package.json without react-scripts
        if (path === "package.json") {
          return JSON.stringify({ name: "test-app" });
        }
        throw new Error(`Unexpected read: ${path}`);
      },
    },
  };

  const { projectConfig, configPath } = await readProjectConfig(testCtx);

  expect(configPath).toBe("convex.json");
  expect(projectConfig).toEqual({
    functions: "convex/",
    node: { externalPackages: [] },
    generateCommonJSApi: false,
    codegen: { staticApi: false, staticDataModel: false },
  });
});

test("parseProjectConfig - preserves unknown properties", async () => {
  // Unknown properties should be preserved for forward/backward compatibility
  await assertParses(
    {
      functions: "convex/",
      unknownField: "some-value",
      futureFeature: {
        nested: "data",
        count: 42,
      },
    },
    {
      functions: "convex/",
      node: { externalPackages: [] },
      generateCommonJSApi: false,
      codegen: { staticApi: false, staticDataModel: false },
      unknownField: "some-value",
      futureFeature: {
        nested: "data",
        count: 42,
      },
    } as any,
  );

  // Unknown properties alongside known ones
  await assertParses(
    {
      functions: "my-functions/",
      generateCommonJSApi: true,
      customMetadata: {
        version: "1.0.0",
        author: "test",
      },
      experimentalFlag: true,
    },
    {
      functions: "my-functions/",
      node: { externalPackages: [] },
      generateCommonJSApi: true,
      codegen: { staticApi: false, staticDataModel: false },
      customMetadata: {
        version: "1.0.0",
        author: "test",
      },
      experimentalFlag: true,
    } as any,
  );
});

test("parseProjectConfig - warns about unknown properties", async () => {
  // Single unknown property
  stderrSpy.mockClear();
  const config1 = await parseProjectConfig(ctx, {
    functions: "convex/",
    unknownField: "value",
  });
  expect(config1.functions).toBe("convex/");
  expect((config1 as any).unknownField).toBe("value");

  // Check that warning was logged
  const stderr1 = stderrSpy.mock.calls.map((call) => call[0]).join("");
  expect(stripVTControlCharacters(stderr1)).toContain(
    "Warning: Unknown property in `convex.json`: `unknownField`",
  );
  expect(stripVTControlCharacters(stderr1)).toContain(
    "These properties will be preserved but are not recognized by this version of Convex",
  );

  // Multiple unknown properties
  stderrSpy.mockClear();
  const config2 = await parseProjectConfig(ctx, {
    functions: "my-functions/",
    customField1: "value1",
    customField2: 42,
    futureFeature: { nested: true },
  });
  expect((config2 as any).customField1).toBe("value1");
  expect((config2 as any).customField2).toBe(42);

  // No warning for known fields only
  stderrSpy.mockClear();
  await parseProjectConfig(ctx, {
    functions: "convex/",
    generateCommonJSApi: true,
  });
  const stderr3 = stderrSpy.mock.calls.map((call) => call[0]).join("");
  expect(stripVTControlCharacters(stderr3)).not.toContain("Warning");
  expect(stripVTControlCharacters(stderr3)).not.toContain("Unknown");

  // No warning for $schema field (used by JSON schema validation)
  stderrSpy.mockClear();
  await parseProjectConfig(ctx, {
    functions: "convex/",
    $schema: "../../../convex/schemas/convex.schema.json",
  });
  const stderr5 = stderrSpy.mock.calls.map((call) => call[0]).join("");
  expect(stripVTControlCharacters(stderr5)).not.toContain("Warning");
  expect(stripVTControlCharacters(stderr5)).not.toContain("Unknown");
});

// AuthKit configuration tests
test("parseProjectConfig - authKit basic valid configs", async () => {
  // Basic config with no settings
  await assertParses(
    {
      functions: "convex/",
      authKit: {
        dev: {},
      },
    },
    {
      functions: "convex/",
      authKit: {
        dev: {},
      },
      codegen: {
        staticApi: false,
        staticDataModel: false,
      },
      generateCommonJSApi: false,
      node: {
        externalPackages: [],
      },
    },
  );

  // Absence of environment config means manual setup
  // No authKit config at all is valid
  await assertParses(
    {
      functions: "convex/",
    },
    {
      functions: "convex/",
      codegen: {
        staticApi: false,
        staticDataModel: false,
      },
      generateCommonJSApi: false,
      node: {
        externalPackages: [],
      },
    },
  );

  // Config with configure settings
  await assertParses(
    {
      functions: "convex/",
      authKit: {
        prod: {
          configure: {
            redirectUris: ["https://example.com/callback"],
            corsOrigins: ["https://example.com"],
          },
        },
      },
    },
    {
      functions: "convex/",
      authKit: {
        prod: {
          configure: {
            redirectUris: ["https://example.com/callback"],
            corsOrigins: ["https://example.com"],
          },
        },
      },
      codegen: {
        staticApi: false,
        staticDataModel: false,
      },
      generateCommonJSApi: false,
      node: {
        externalPackages: [],
      },
    },
  );

  // Full config with all deployment types
  await assertParses(
    {
      functions: "convex/",
      authKit: {
        dev: {
          configure: {
            redirectUris: ["http://localhost:5173/callback"],
            corsOrigins: ["http://localhost:5173"],
          },
          localEnvVars: {
            VITE_WORKOS_CLIENT_ID: "${authEnv.WORKOS_CLIENT_ID}",
          },
        },
        preview: {
          configure: {
            redirectUris: ["${buildEnv.VERCEL_BRANCH_URL}/callback"],
            corsOrigins: ["${buildEnv.VERCEL_BRANCH_URL}"],
          },
        },
        prod: {
          environmentType: "production",
          configure: {
            redirectUris: ["https://example.com/callback"],
            corsOrigins: ["https://example.com"],
          },
        },
      },
    },
    {
      functions: "convex/",
      authKit: {
        dev: {
          configure: {
            redirectUris: ["http://localhost:5173/callback"],
            corsOrigins: ["http://localhost:5173"],
          },
          localEnvVars: {
            VITE_WORKOS_CLIENT_ID: "${authEnv.WORKOS_CLIENT_ID}",
          },
        },
        preview: {
          configure: {
            redirectUris: ["${buildEnv.VERCEL_BRANCH_URL}/callback"],
            corsOrigins: ["${buildEnv.VERCEL_BRANCH_URL}"],
          },
        },
        prod: {
          environmentType: "production",
          configure: {
            redirectUris: ["https://example.com/callback"],
            corsOrigins: ["https://example.com"],
          },
        },
      },
      codegen: {
        staticApi: false,
        staticDataModel: false,
      },
      generateCommonJSApi: false,
      node: {
        externalPackages: [],
      },
    },
  );
});

test("parseProjectConfig - authKit validation errors", async () => {
  // environmentType only allowed in prod
  await assertParseError(
    {
      functions: "convex/",
      authKit: {
        dev: {
          environmentType: "development",
        },
      },
    },
    "✖ `authKit.environmentType` in `convex.json`: authKit.environmentType is only allowed in the prod section\n",
  );

  // Invalid environmentType value
  await assertParseError(
    {
      functions: "convex/",
      authKit: {
        prod: {
          environmentType: "invalid" as any,
        },
      },
    },
    "✖ `authKit.prod.environmentType` in `convex.json`: Invalid enum value. Expected 'development' | 'staging' | 'production', received 'invalid'\n",
  );

  // authEnv references are allowed in localEnvVars
  await assertParses(
    {
      functions: "convex/",
      authKit: {
        dev: {
          localEnvVars: {
            WORKOS_CLIENT_ID: "${authEnv.WORKOS_CLIENT_ID}",
          },
        },
      },
    },
    {
      functions: "convex/",
      authKit: {
        dev: {
          localEnvVars: {
            WORKOS_CLIENT_ID: "${authEnv.WORKOS_CLIENT_ID}",
          },
        },
      },
      codegen: {
        staticApi: false,
        staticDataModel: false,
      },
      generateCommonJSApi: false,
      node: {
        externalPackages: [],
      },
    },
  );

  // buildEnv references are also allowed in localEnvVars
  await assertParses(
    {
      functions: "convex/",
      authKit: {
        dev: {
          localEnvVars: {
            WORKOS_CLIENT_ID: "${buildEnv.MY_WORKOS_CLIENT_ID}",
          },
        },
      },
    },
    {
      functions: "convex/",
      authKit: {
        dev: {
          localEnvVars: {
            WORKOS_CLIENT_ID: "${buildEnv.MY_WORKOS_CLIENT_ID}",
          },
        },
      },
      codegen: {
        staticApi: false,
        staticDataModel: false,
      },
      generateCommonJSApi: false,
      node: {
        externalPackages: [],
      },
    },
  );

  // authEnv references are allowed in configure
  await assertParses(
    {
      functions: "convex/",
      authKit: {
        dev: {
          configure: {
            redirectUris: ["${authEnv.WORKOS_CLIENT_ID}/callback"],
          },
        },
      },
    },
    {
      functions: "convex/",
      authKit: {
        dev: {
          configure: {
            redirectUris: ["${authEnv.WORKOS_CLIENT_ID}/callback"],
          },
        },
      },
      codegen: {
        staticApi: false,
        staticDataModel: false,
      },
      generateCommonJSApi: false,
      node: {
        externalPackages: [],
      },
    },
  );

  // Mixed buildEnv and authEnv references are allowed
  await assertParses(
    {
      functions: "convex/",
      authKit: {
        dev: {
          localEnvVars: {
            WORKOS_CLIENT_ID: "${authEnv.WORKOS_CLIENT_ID}",
            WORKOS_API_KEY: "${authEnv.WORKOS_API_KEY}",
          },
          configure: {
            redirectUris: ["${buildEnv.VERCEL_URL}/callback"],
          },
        },
      },
    },
    {
      functions: "convex/",
      authKit: {
        dev: {
          localEnvVars: {
            WORKOS_CLIENT_ID: "${authEnv.WORKOS_CLIENT_ID}",
            WORKOS_API_KEY: "${authEnv.WORKOS_API_KEY}",
          },
          configure: {
            redirectUris: ["${buildEnv.VERCEL_URL}/callback"],
          },
        },
      },
      codegen: {
        staticApi: false,
        staticDataModel: false,
      },
      generateCommonJSApi: false,
      node: {
        externalPackages: [],
      },
    },
  );
});

test("parseProjectConfig - authKit preview and prod restrictions", async () => {
  // Preview with localEnvVars should fail
  await assertParseError(
    {
      authKit: {
        preview: {
          localEnvVars: {
            WORKOS_CLIENT_ID: "${buildEnv.WORKOS_CLIENT_ID}",
          },
        },
      },
    },
    "✖ `authKit.localEnvVars` in `convex.json`: authKit.localEnvVars is only supported for dev deployments. Preview and prod deployments must configure environment variables directly in the deployment platform.\n",
  );

  // Prod with localEnvVars should fail
  await assertParseError(
    {
      authKit: {
        prod: {
          localEnvVars: {
            WORKOS_CLIENT_ID: "${buildEnv.WORKOS_CLIENT_ID}",
          },
        },
      },
    },
    "✖ `authKit.localEnvVars` in `convex.json`: authKit.localEnvVars is only supported for dev deployments. Preview and prod deployments must configure environment variables directly in the deployment platform.\n",
  );

  // Dev with localEnvVars should still work
  await assertParses({
    functions: "convex/",
    authKit: {
      dev: {
        localEnvVars: {
          WORKOS_CLIENT_ID: "${authEnv.WORKOS_CLIENT_ID}",
        },
      },
    },
    codegen: {
      staticApi: false,
      staticDataModel: false,
    },
    generateCommonJSApi: false,
    node: {
      externalPackages: [],
    },
  });

  // Preview with just configure should work (will use env vars)
  await assertParses(
    {
      functions: "convex/",
      authKit: {
        preview: {
          configure: {
            redirectUris: ["https://preview.example.com/callback"],
          },
        },
      },
    },
    {
      functions: "convex/",
      authKit: {
        preview: {
          configure: {
            redirectUris: ["https://preview.example.com/callback"],
          },
        },
      },
      codegen: {
        staticApi: false,
        staticDataModel: false,
      },
      generateCommonJSApi: false,
      node: {
        externalPackages: [],
      },
    },
  );

  // Prod without localEnvVars should work
  await assertParses(
    {
      functions: "convex/",
      authKit: {
        prod: {
          environmentType: "production",
        },
      },
    },
    {
      functions: "convex/",
      authKit: {
        prod: {
          environmentType: "production",
        },
      },
      codegen: {
        staticApi: false,
        staticDataModel: false,
      },
      generateCommonJSApi: false,
      node: {
        externalPackages: [],
      },
    },
  );
});
