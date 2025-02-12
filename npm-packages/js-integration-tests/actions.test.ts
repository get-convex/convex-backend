// This file can be combined with ./basic.test.ts once these APIs are public.

import { ConvexHttpClient } from "convex/browser";
import { ConvexReactClient } from "convex/react";
import { awaitQueryResult, opts } from "./test_helpers";
import { api, internal } from "./convex/_generated/api";
import { adminKey, deploymentUrl, siteUrl } from "./common";

describe("HTTPClient", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });

  test("Run an action", async () => {
    const result = await httpClient.action(api.actions.simple.hello, {
      somebody: "Presley",
    });
    expect(result).toStrictEqual("Aloha, Presley!");
  });

  test("Hydrates return value", async () => {
    const result = await httpClient.action(api.actions.simple.returnInt64);
    expect(result).toEqual(BigInt(1));
  });

  test("Action failure", async () => {
    let err: null | Error = null;
    try {
      await httpClient.action(api.actions.simple.userError);
    } catch (e) {
      err = e as Error;
    }
    expect(err).not.toBeNull();
    expect(err!.toString()).toMatch(/I failed you!/);
  });

  test("action that deadlocks", async () => {
    await expect(
      httpClient.action(api.actions.simple.deadlock),
    ).rejects.toThrow(
      /.*Action `deadlock` execution timed out \(maximum duration/s,
    );
  }, 25000);

  test("action scheduling", async () => {
    const result = await httpClient.action(api.actions.simple.scheduling);
    // Regression test where `scheduler.runAfter` returned an object with the jobId
    // instead of the jobId itself.
    expect(typeof result).toBe("string");
  });

  test("convex cloud url env vars", async () => {
    const result = await httpClient.action(api.actions.simple.convexCloud);
    expect(result).toEqual(deploymentUrl);

    const result2 = await httpClient.action(api.actions.simple.convexSite);
    expect(result2).toEqual(siteUrl);
  });

  test("Run an internal action", async () => {
    await expect(async () => {
      await httpClient.action(internal.actions.simple.internalUhOh as any);
    }).rejects.toThrow(
      "Could not find public function for 'actions/simple:internalUhOh'. Did you forget to run `npx convex dev` or `npx convex deploy`?",
    );
  });

  test("Action calls a function with big args", async () => {
    await httpClient.action(api.actions.simple.actionCallsWithBigArgument);
  });
});

describe("ConvexReactClient", () => {
  let reactClient: ConvexReactClient;
  beforeEach(() => {
    reactClient = new ConvexReactClient(deploymentUrl, opts);
  });
  afterEach(async () => {
    await reactClient.mutation(api.cleanUp.default);
    await reactClient.close();
  });

  test("Run a nested query action with auth", async () => {
    reactClient.setAdminAuth(adminKey, { issuer: "abc", subject: "def" });
    // This action calls a query that returns auth
    const result = await reactClient.action(api.actions.auth.q);
    expect(result).toStrictEqual({
      tokenIdentifier: "abc|def",
      issuer: "abc",
      subject: "def",
    });
  });

  test("Run a nested mutation action with auth", async () => {
    reactClient.setAdminAuth(adminKey, { issuer: "abc", subject: "def" });
    // This action calls a mutation that returns auth
    const result = await reactClient.action(api.actions.auth.m);
    expect(result).toStrictEqual({
      tokenIdentifier: "abc|def",
      issuer: "abc",
      subject: "def",
    });
  });

  test("Run a nested scheduler action with auth", async () => {
    reactClient.setAdminAuth(adminKey, { issuer: "abc", subject: "def" });
    // This action schedules an action that stores an object with the property `foundUser`
    await reactClient.action(api.actions.auth.s);

    // Wait for the schedule function to complete by waiting for a query relying on the scheduled data to update
    const watch = reactClient.watchQuery(api.findObject.default, {});
    const found: any = await awaitQueryResult(watch, (doc) => doc !== null);

    // Auth is not propogated to scheduled jobs.
    expect(found?.foundUser).toStrictEqual(false);
  });

  test("Run a nested scheduler mutation with auth", async () => {
    reactClient.setAdminAuth(adminKey, { issuer: "abc", subject: "def" });
    // This action schedules an action that stores an object with the property `foundUser`
    await reactClient.mutation(api.auth.s);

    // Wait for the schedule function to complete by waiting for a query relying on the scheduled data to update
    const watch = reactClient.watchQuery(api.findObject.default, {});
    const found: any = await awaitQueryResult(watch, (doc) => doc !== null);

    // Auth is not propogated to scheduled jobs.
    expect(found?.foundUser).toStrictEqual(false);
  });
});
