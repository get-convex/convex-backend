import assert from "assert";
import { v4 as uuidv4 } from "uuid";
import { jsonToConvex } from "convex/values";
import {
  ExecuteResponseInner,
  executeInner,
  setupConsole,
} from "../src/executor.js";
import { Syscalls, SyscallsImpl } from "../src/syscalls.js";
import {
  createPackageJsonIfMissing,
  maybeDownloadAndLinkPackages,
} from "../src/source_package.js";
import { EnvironmentVariable, FunctionName } from "../src/convex.js";
import * as fs from "node:fs";
import archiver from "archiver";
import { hashFromFile } from "../src/build_deps";
import path from "node:path";
import os from "node:os";
import { randomUUID } from "crypto";
import { Writable } from "stream";

type NewType = FunctionName;

async function executeWrapper(
  modulePath: string,
  name: NewType,
  args: string,
  environmentVariables: EnvironmentVariable[],
  syscalls?: Syscalls,
) {
  createPackageJsonIfMissing(__dirname);
  const saved = console;
  const timeoutSecs = 300;
  const logLines: string[] = [];
  const responseStream = new Writable({
    write: (chunk, _encoding, callback) => {
      const chunkJson = JSON.parse(chunk);
      if (chunkJson.kind === "LogLine") {
        logLines.push(chunkJson.data);
      }
      callback();
    },
  });
  try {
    setupConsole(responseStream);
    const requestId = uuidv4();
    const response = await executeInner(
      requestId,
      __dirname,
      modulePath,
      name,
      args,
      environmentVariables,
      timeoutSecs,
      syscalls ??
        new SyscallsImpl(
          { canonicalizedPath: "", function: "" },
          requestId,
          "",
          "",
          null,
          null,
          null,
          {
            requestId: randomUUID(),
            parentScheduledJob: null,
          },
        ),
    );
    return { response, logLines };
  } finally {
    globalThis.console = saved;
  }
}

const printResponses = true;
function printResponse(response: ExecuteResponseInner) {
  if (!printResponses) {
    return;
  }
  if (response.type === "success") {
    console.log(`SUCCESS -> ${response.udfReturn}`);
    for (const logLine of response.logLines) {
      console.log(`[log] ${logLine}`);
    }
  } else {
    console.log(`ERROR -> ${response.message}`);
    for (const frame of response.frames ?? []) {
      console.log(
        `  ${frame.functionName ?? "[unknown]"} @ ${frame.fileName}:${
          frame.lineNumber
        }`,
      );
    }
    for (const logLine of response.logLines ?? []) {
      console.log(`[log] ${logLine}`);
    }
  }
}

async function expectFailure(fn: () => Promise<unknown>): Promise<Error> {
  let error = null;
  try {
    await fn();
  } catch (e: any) {
    error = e;
  }
  if (!error) {
    throw new Error(`Unexpected success`);
  }
  return error;
}

/**
 * Regression test for
 * `request for './transitive.js' is not yet fulfilled`
 * resolved in #11415 41f3fdbd270b1250cfc00e274c0a93810733cc26
 *
 *      diamond.js
 *       |     |
 *  left.js   right.js
 *       |     |
 *      shared.js
 *          |
 *     transitive.js
 */
async function test_diamond() {
  const { response, logLines } = await executeWrapper(
    "diamond.js",
    "default",
    "",
    [],
  );

  printResponse(response);
  if (response.type !== "success") {
    throw new Error(`Unexpected error`);
  }
  assert.deepEqual(response.udfReturn, "1");
  assert.deepEqual(logLines, []);
}

/**
 * Test for circular dependencies.
 */
async function test_cyclic() {
  const { response, logLines } = await executeWrapper(
    "cyclic1.js",
    "default",
    "",
    [],
  );

  printResponse(response);
  if (response.type !== "success") {
    throw new Error(`Unexpected error`);
  }
  assert.deepEqual(response.udfReturn, "1");
  assert.deepEqual(logLines, []);
}

async function test_execute_success() {
  const { response, logLines } = await executeWrapper(
    "b.js",
    "default",
    "5",
    [],
  );

  printResponse(response);
  if (response.type !== "success") {
    throw new Error(`Unexpected error`);
  }
  assert.deepEqual(response.udfReturn, "8");
  assert.deepEqual(logLines, ["[LOG] 'Computing...'"]);
}

async function test_execute_env_var() {
  // Initialize and execute once.
  let result = await executeWrapper("d.js", "default", "", [
    { name: "GLOBAL_SCOPE_VAR", value: "982" },
  ]);
  let response = result.response;
  let logLines = result.logLines;

  printResponse(response);
  if (response.type !== "success") {
    throw new Error(`Unexpected error`);
  }
  assert.deepEqual(response.udfReturn, "982");
  assert.deepEqual(logLines, []);

  // Call agin with same env variables.
  result = await executeWrapper("d.js", "default", "", [
    { name: "GLOBAL_SCOPE_VAR", value: "982" },
  ]);
  response = result.response;
  logLines = result.logLines;

  printResponse(response);
  if (response.type !== "success") {
    throw new Error(`Unexpected error`);
  }
  assert.deepEqual(response.udfReturn, "982");
  assert.deepEqual(logLines, []);

  // Call with different env variables. Should recompile.
  result = await executeWrapper("d.js", "default", "", [
    { name: "GLOBAL_SCOPE_VAR", value: "329" },
  ]);
  response = result.response;
  logLines = result.logLines;

  printResponse(response);
  if (response.type !== "success") {
    throw new Error(`Unexpected error`);
  }
  assert.deepEqual(response.udfReturn, "329");
  assert.deepEqual(logLines, []);
}

// Tests that env vars don't leak into the Node
// environment.
async function test_execute_env_var_sanitanization() {
  // Set a env variable from this outer environment.
  process.env.GLOBAL_SCOPE_VAR = "secret";
  const error = await expectFailure(() =>
    executeWrapper("d.js", "default", "", []),
  );
  assert.deepEqual(
    error.message,
    "Action `default` did not return a string (returned `undefined`)",
  );
}

async function test_execute_failure() {
  const { response } = await executeWrapper("b.js", "throwError", "5", []);

  printResponse(response);
  if (response.type !== "error") {
    throw new Error(`Unexpected success`);
  }
  assert.deepEqual(response.message, "such is life");
  assert.notDeepEqual(response.frames, []);
}

async function test_execute_missing_module() {
  const error = await expectFailure(() =>
    executeWrapper("zzz.js", "fibonacci", "very-important-arg", []),
  );
  assert(error.message.includes("Cannot find module"));
}

async function test_execute_non_convex_action() {
  const error = await expectFailure(() =>
    executeWrapper("a.js", "fibonacci", "very-important-arg", []),
  );
  assert.deepEqual(
    error.message,
    "`fibonacci` wasn't registered as a Convex action in `a.js`",
  );
}

async function test_execute_action_returns_number() {
  const error = await expectFailure(() =>
    executeWrapper("b.js", "getNumber", "5", []),
  );
  assert.deepEqual(
    error.message,
    "Action `getNumber` did not return a string (returned `8`)",
  );
}

async function test_execute_syscall() {
  let syscallCount = 0;
  const mockSyscalls = {
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    syscall(op: string, jsonArgs: string): string {
      assert(false, "sync syscalls not allowed");
    },
    asyncSyscall(op: string, jsonArgs: string): Promise<string> {
      JSON.parse(jsonArgs);
      switch (op) {
        case "1.0/actions/query":
          syscallCount++;
          return Promise.resolve(JSON.stringify([7, 8]));
        case "1.0/actions/mutation":
          syscallCount++;
          return Promise.resolve(JSON.stringify(18));
        default:
          throw new Error(`Unknown operation ${op}`);
      }
    },
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    asyncJsSyscall(op: string, args: Record<string, any>): Promise<any> {
      throw new Error("asyncJsSyscall not allowed");
    },
    // eslint-disable-next-line @typescript-eslint/no-empty-function
    assertNoPendingSyscalls() {},
  };
  const { response, logLines } = await executeWrapper(
    "c.js",
    "default",
    "[]",
    [],
    mockSyscalls,
  );
  printResponse(response);
  if (response.type !== "success") {
    throw new Error(`Unexpected error`);
  }
  assert.deepEqual(response.udfReturn, "[7,8]");
  assert.deepEqual(logLines, []);
  assert.deepEqual(syscallCount, 2);
}

async function test_execute_invalid_syscall() {
  const { response } = await executeWrapper(
    "c.js",
    "default",
    "[]",
    [],
    new SyscallsImpl(
      { canonicalizedPath: "", function: "" },
      "invalid-uuid",
      "",
      "",
      null,
      null,
      null,
      {
        requestId: randomUUID(),
        parentScheduledJob: null,
      },
    ),
  );
  printResponse(response);
  if (response.type !== "error") {
    throw new Error(`Unexpected success`);
  }
  assert.deepEqual(
    response.message,
    "Leftover state detected. This typically happens if there are dangling " +
      "promises from a previous request. Did you forget to await your promises?",
  );
  assert.notDeepEqual(response.frames, []);
}

async function runExample(example: string) {
  const { response } = await executeWrapper(
    "/third_party.js",
    example,
    "[]",
    [],
  );
  printResponse(response);
  assert.equal(response.type, "success");
  return jsonToConvex(JSON.parse((response as any).udfReturn));
}

/* Disable until fixed (CX-3699)
async function test_execute_stripe() {
  const checkoutUrl = (await runExample("stripeExample")) as string;
  assert.equal(checkoutUrl.startsWith("https://checkout.stripe.com"), true);
}
*/

async function test_filename() {
  const filename = (await runExample("testFilename")) as string;
  assert(
    filename.includes(
      "/npm-packages/node-executor/dist/tests/integration-test.cjs",
    ),
  );
}

async function test_dirname() {
  const dirname = (await runExample("testDirname")) as string;
  assert(dirname.includes("/npm-packages/node-executor/dist/tests"));
}

async function test_modules() {
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  const module = require("module");
  assert(module.builtinModules.includes("assert"));
  assert(module.builtinModules.includes("async_hooks"));
  assert(module.builtinModules.includes("process"));
  assert(module.builtinModules.includes("trace_events"));

  assert(module.isBuiltin("async_hooks"));
  assert(module.isBuiltin("node:async_hooks"));
  assert(!module.isBuiltin("sync_hooks"));
  assert(!module.isBuiltin("node:sync_hooks"));
}

// Make sure that instanceof works as expected. Used to be a big deal when
// we used vm.Module, but works ok when leverage Node.js
async function test_contexts() {
  const { response, logLines } = await executeWrapper(
    "contexts.js",
    "default",
    "",
    [],
  );

  printResponse(response);
  if (response.type !== "success") {
    throw new Error(`Unexpected error`);
  }
  for (const [name, succeeded] of Object.entries(
    JSON.parse(response.udfReturn) as Record<string, boolean>,
  )) {
    assert.deepEqual(succeeded, true, name);
  }
  assert.deepEqual(logLines, []);
}

async function test_download() {
  // Write the source zip file with metadata.json and modules/
  const sourceOutput = fs.createWriteStream(`${__dirname}/source.zip`);
  const sourceZip = archiver("zip");
  const sourceStream = sourceZip.pipe(sourceOutput);
  sourceZip.directory(`${__dirname}/modules`, "modules");
  sourceZip.file(`${__dirname}/metadata.json`, { name: "metadata.json" });
  sourceZip.finalize();
  await new Promise((resolve) => {
    sourceStream
      .on("finish", () => {
        resolve(null);
      })
      .on("error", (err) => {
        throw err;
      });
  });
  const sourceHash = await hashFromFile(`${__dirname}/source.zip`);

  // Write the external deps zip file
  const externalDepsOutput = fs.createWriteStream(
    `${__dirname}/external_modules.zip`,
  );
  const externalDepsZip = archiver("zip");
  const externalDepsStream = externalDepsZip.pipe(externalDepsOutput);
  externalDepsZip.directory(`${__dirname}/external_modules`, "node_modules");
  externalDepsZip.finalize();
  await new Promise((resolve) => {
    externalDepsStream
      .on("finish", () => {
        resolve(null);
      })
      .on("error", (err) => {
        throw err;
      });
  });
  const externalDepsHash = await hashFromFile(
    `${__dirname}/external_modules.zip`,
  );

  const sourceHashDigest = sourceHash.digest().toString("base64url");
  const sourcePackage = {
    uri: `file:${__dirname}/source.zip`,
    key: "test_modules_key",
    sha256: sourceHashDigest,
    bundled_source: {
      uri: `file:${__dirname}/source.zip`,
      key: "test_modules_key",
      sha256: sourceHashDigest,
    },
    external_deps: {
      uri: `file:${__dirname}/external_modules.zip`,
      key: "test_external_deps_key",
      sha256: externalDepsHash.digest().toString("base64url"),
    },
  };

  const local = await maybeDownloadAndLinkPackages(sourcePackage);
  const sourceDir = path.join(os.tmpdir(), `source/test_modules_key`);
  assert.equal(local.dir, sourceDir);
  assert.equal(local.modules.has("a.js"), true);
  // Non-node modules
  assert.equal(local.modules.has("third_party.js"), false);
  assert.equal(local.modules.has("d.js"), false);

  // Assert external deps package exists after download
  const externalDepsDir = path.join(
    os.tmpdir(),
    `external_deps/test_external_deps_key`,
  );
  assert.equal(fs.existsSync(path.join(externalDepsDir, "node_modules")), true);
}

async function test_logging() {
  let result = await executeWrapper("logging.js", "logSome", "[]", []);
  let logLines = result.logLines;
  assert.strictEqual(logLines.length, 40);
  assert.strictEqual(logLines[0], "[LOG] 'Hello'");

  result = await executeWrapper("logging.js", "logTooManyLines", "[]", []);
  logLines = result.logLines;
  assert.strictEqual(logLines.length, 257);
  assert.strictEqual(logLines[0], "[LOG] 'Hello'");
  assert(
    logLines[256].includes(
      "Log overflow (maximum 256). Remaining log lines omitted.",
    ),
  );

  result = await executeWrapper("logging.js", "logOverTotalLength", "[]", []);
  logLines = result.logLines;
  assert.strictEqual(logLines.length, 32);
  assert(
    logLines[logLines.length - 1].includes(
      "[ERROR] Log overflow (maximum 1M characters). Remaining log lines omitted.",
    ),
  );
}

(async () => {
  await test_diamond();
  await test_contexts();

  // Expected failure: our current algorithm can't do this
  let failed = false;
  try {
    await test_cyclic();
  } catch (e) {
    console.log(e);
    failed = true;
  }
  if (!failed) {
    assert.fail("Expected test to fail");
  }

  await test_execute_success();
  await test_execute_env_var();
  await test_execute_env_var_sanitanization();
  await test_execute_failure();
  await test_execute_missing_module();
  await test_execute_non_convex_action();
  await test_execute_action_returns_number();
  await test_execute_syscall();
  await test_execute_invalid_syscall();
  // Disable until fixed (CX-3699)
  // await test_execute_stripe();
  await test_filename();
  await test_dirname();
  await test_modules();
  await test_download();
  await test_logging();
})();
