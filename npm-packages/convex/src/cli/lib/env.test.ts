import { beforeEach, describe, expect, test, vi } from "vitest";
import type { Context } from "../../bundler/context.js";
import { envList, type EnvVar, type EnvVarBackend } from "./env.js";

const logMocks = vi.hoisted(() => ({
  logFailure: vi.fn(),
  logFinishedStep: vi.fn(),
  logMessage: vi.fn(),
  logOutput: vi.fn(),
}));

vi.mock("../../bundler/log.js", () => ({
  logFailure: logMocks.logFailure,
  logFinishedStep: logMocks.logFinishedStep,
  logMessage: logMocks.logMessage,
  logOutput: logMocks.logOutput,
}));

function fakeBackend(envs: EnvVar[]): EnvVarBackend {
  return {
    get() {
      return Promise.resolve(null);
    },
    list() {
      return Promise.resolve(envs);
    },
    update() {
      return Promise.reject(new Error("Unexpected update"));
    },
    notice: "",
  };
}

describe("envList", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test("prints only names without values or warnings when namesOnly is true", async () => {
    await envList(
      {} as Context,
      fakeBackend([
        { name: "PLAIN", value: "plain-value" },
        { name: "DANGEROUS", value: "secret'value#fragment" },
      ]),
      { namesOnly: true },
    );

    expect(logMocks.logOutput.mock.calls).toEqual([["PLAIN"], ["DANGEROUS"]]);
    expect(logMocks.logMessage).not.toHaveBeenCalled();

    const emitted = [
      ...logMocks.logOutput.mock.calls.flat(),
      ...logMocks.logMessage.mock.calls.flat(),
    ].join("\n");
    expect(emitted).not.toContain("plain-value");
    expect(emitted).not.toContain("secret'value#fragment");
    expect(emitted).not.toContain("Warning (");
  });
});
