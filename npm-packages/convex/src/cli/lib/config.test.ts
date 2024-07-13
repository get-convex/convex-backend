import { vi, test, expect } from "vitest";
import { parseProjectConfig } from "./config.js";
import { oneoffContext } from "../../bundler/context.js";

test("parseProjectConfig", async () => {
  // Make a context that throws on crashes so we can detect them.
  const ctx = {
    ...oneoffContext,
    crash: () => {
      // eslint-disable-next-line no-restricted-syntax
      throw new Error();
    },
  };
  const stderrSpy = vi.spyOn(process.stderr, "write").mockImplementation(() => {
    // Do nothing
    return true;
  });
  const assertParses = async (inp: any) => {
    expect(await parseProjectConfig(ctx, inp)).toEqual(inp);
  };
  const assertParseError = async (inp: any, err: string) => {
    await expect(parseProjectConfig(ctx, inp)).rejects.toThrow();
    expect(stderrSpy).toHaveBeenCalledWith(err);
  };

  await assertParses({
    team: "team",
    project: "proj",
    prodUrl: "prodUrl",
    functions: "functions/",
  });

  await assertParses({
    team: "team",
    project: "proj",
    prodUrl: "prodUrl",
    functions: "functions/",
    authInfos: [],
  });

  await assertParses({
    team: "team",
    project: "proj",
    prodUrl: "prodUrl",
    functions: "functions/",
    authInfos: [
      {
        applicationID: "hello",
        domain: "world",
      },
    ],
  });

  await assertParseError(
    {
      team: "team",
      project: "proj",
      prodUrl: "prodUrl",
      functions: "functions/",
      authInfo: [{}],
    },
    "Expected `authInfo` in `convex.json` to be of type AuthInfo[]\n",
  );
});
