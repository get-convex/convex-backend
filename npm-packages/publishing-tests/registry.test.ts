import { ChildProcess, fork, execSync } from "child_process";
import path from "path";
import fs from "fs";
import os from "os";
import { fileURLToPath } from "url";
import Module from "node:module";
import getPort from "get-port";
import {
  beforeAll,
  afterAll,
  test,
  expect,
  beforeEach,
  afterEach,
} from "vitest";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const convexDir = path.resolve(path.join(__dirname, "..", "convex"));
const require = Module.createRequire(import.meta.url);

function runRegistry(args: string[] = []): Promise<ChildProcess> {
  return new Promise((resolve, reject) => {
    const childFork = fork(require.resolve("verdaccio/bin/verdaccio"), args);

    childFork.on("message", (msg: { verdaccio_started: boolean }) => {
      if (msg.verdaccio_started) {
        resolve(childFork);
      }
    });

    childFork.on("error", (err: any) => reject([err]));
    childFork.on("disconnect", (err: any) => reject([err]));
  });
}

function login(user: string, password: string, port: string) {
  execSync(
    `npx npm-cli-login -u ${user} -p ${password} -e test@domain.test -r http://localhost:${port}`,
  );
}

function runNpmCommand(command: string, port: string): string {
  const buffer = execSync(
    `npm ${command} --registry http://localhost:${port} --json`,
    { cwd: convexDir },
  );
  return buffer.toString();
}

function runNpmInfo(pkg: string, port: string): unknown {
  const buffer = runNpmCommand(`info ${pkg}`, port);
  return JSON.parse(buffer.toString());
}

function runNpmPack(port: string) {
  runNpmCommand(`pack `, port);
}

function runNpmPublish(port: string, tarball: string) {
  const buffer = runNpmCommand(
    `publish "${tarball}" --access=public --json`,
    port,
  );
  return JSON.parse(buffer.toString());
}

function getOnlyTarball(dirname: string) {
  const files = fs.readdirSync(dirname);
  const tarballs = files.filter((f) => f.endsWith(".tgz"));
  if (tarballs.length < 1) throw new Error("No tarball found.");
  if (tarballs.length > 1) throw new Error("Multiple tarballs found.");
  return path.join(dirname, tarballs[0]);
}

let child: ChildProcess;
let port = 4000;
let tmpDir: string;

beforeAll(async () => {
  port = await getPort();
  child = await runRegistry(["-c", "./verdaccio.yaml", "-l", `${port}`]);
});

afterAll(() => {
  child.kill();
});

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "convex-publish-test-"));
});

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true });
});

test("publishing convex", { timeout: 2 * 60 * 1000 }, async () => {
  login("foo", "bar", port.toString());

  // This should be a `just rush publish --pack --include-all --publish`
  // but this is easier to test.
  runNpmPack(port.toString());
  const packedOutput = getOnlyTarball(convexDir);
  const tarball = path.join(tmpDir, "package.tgz");
  fs.renameSync(packedOutput, tarball);
  const object = runNpmPublish(port.toString(), tarball);
  console.log(object);
  const originalPackageJson = JSON.parse(
    fs.readFileSync(path.join(convexDir, "package.json"), {
      encoding: "utf-8",
    }),
  );
  const pkgName = `${originalPackageJson.name}@${originalPackageJson.version}`;
  const info = runNpmInfo(pkgName, port.toString()) as any;

  // Check that the published package has the correct version
  expect(info.name).toBe(originalPackageJson.name);
  expect(info.version).toBe(originalPackageJson.version);

  // In the same test (to avoid packing again) check out this tarball
  execSync(`tar xzf ${tarball}`, { cwd: tmpDir });
  const packageJson = JSON.parse(
    fs.readFileSync(path.join(tmpDir, "package/package.json"), {
      encoding: "utf-8",
    }),
  );
  // Binary should be the built one, not the tsx one.
  expect(packageJson.bin["convex-bundled"]).toBeUndefined();
  expect(packageJson.bin["convex"]).toBe("bin/main.js");

  // Stub package.json files should not point to internal types
  const browserStubJson = JSON.parse(
    fs.readFileSync(path.join(tmpDir, "package/browser/package.json"), {
      encoding: "utf-8",
    }),
  );
  expect(browserStubJson.types).not.contains("internal");
});
