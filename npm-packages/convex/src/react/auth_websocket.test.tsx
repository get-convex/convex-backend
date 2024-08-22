/**
 * @vitest-environment custom-vitest-environment.ts
 */
import { expect, vi, test, describe } from "vitest";
import jwtEncode from "jwt-encode";
import {
  nodeWebSocket,
  withInMemoryWebSocket,
} from "../browser/sync/client_node_test_helpers.js";
import { ConvexReactClient } from "./index.js";
import waitForExpect from "wait-for-expect";
import { anyApi } from "../server/index.js";
import { Long } from "../browser/long.js";

const testReactClient = (address: string) =>
  new ConvexReactClient(address, {
    webSocketConstructor: nodeWebSocket,
    unsavedChangesWarning: false,
  });

// Disabled due to flakes in CI
// https://linear.app/convex/issue/ENG-7052/re-enable-auth-websocket-client-tests

// On Linux these can retry forever due to EADDRINUSE so run then sequentially.
describe.sequential.skip("auth websocket tests", () => {
  // This is the path usually taken on page load after a user logged in,
  // with a constant token provider.
  test("Authenticate via valid static token", async () => {
    await withInMemoryWebSocket(async ({ address, receive, send }) => {
      const client = testReactClient(address);

      const tokenFetcher = vi.fn(async () =>
        jwtEncode({ iat: 1234500, exp: 1244500 }, "secret"),
      );
      const onAuthChange = vi.fn();
      client.setAuth(tokenFetcher, onAuthChange);

      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("Authenticate");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      const querySetVersion = client.sync["remoteQuerySet"]["version"];

      send({
        type: "Transition",
        startVersion: querySetVersion,
        endVersion: {
          ...querySetVersion,
          // Client started at 0
          // Good token advanced to 1
          identity: 1,
        },
        modifications: [],
      });

      await waitForExpect(() => {
        expect(onAuthChange).toHaveBeenCalledTimes(1);
      });
      await client.close();

      expect(tokenFetcher).toHaveBeenCalledWith({ forceRefreshToken: false });
      expect(onAuthChange).toHaveBeenCalledTimes(1);
      expect(onAuthChange).toHaveBeenCalledWith(true);
    });
  });

  // This happens when a user opens a page after their cached token expired
  test("Reauthenticate after token expiration without versioning", async () => {
    await testRauthenticationOnInvalidTokenSucceeds(undefined);
  });
  test("Reauthenticate after token expiration with versioning", async () => {
    await testRauthenticationOnInvalidTokenSucceeds(0);
  });

  async function testRauthenticationOnInvalidTokenSucceeds(
    authErrorBaseVersion: number | undefined,
  ) {
    await withInMemoryWebSocket(async ({ address, receive, send, close }) => {
      const client = testReactClient(address);

      let token = jwtEncode({ iat: 1234500, exp: 1244500 }, "wobabloobla");
      const fetchToken = async () => token;
      const tokenFetcher = vi.fn(fetchToken);
      const onAuthChange = vi.fn();
      client.setAuth(tokenFetcher, onAuthChange);

      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("Authenticate");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      // Token must change, otherwise client will not try to reauthenticate
      token = jwtEncode({ iat: 1234500, exp: 1244500 }, "secret");

      send({
        type: "AuthError",
        error: "bla",
        baseVersion: authErrorBaseVersion,
      });
      close();

      // The client reconnects automatically
      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("Authenticate");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      const querySetVersion = client.sync["remoteQuerySet"]["version"];

      // Server accepts new token
      send({
        type: "Transition",
        startVersion: querySetVersion,
        endVersion: {
          ...querySetVersion,
          identity: querySetVersion.identity + 1,
        },
        modifications: [],
      });

      await waitForExpect(() => {
        expect(onAuthChange).toHaveBeenCalledTimes(1);
      });
      await client.close();

      expect(tokenFetcher).toHaveBeenNthCalledWith(1, {
        forceRefreshToken: false,
      });
      expect(tokenFetcher).toHaveBeenNthCalledWith(2, {
        forceRefreshToken: true,
      });
      expect(onAuthChange).toHaveBeenCalledWith(true);
    });
  }

  // This happens when a user opens a page and they cannot
  // fetch from a cache (say due to Prevent Cross Site tracking)
  test("Reauthenticate after token cache failure", async () => {
    await withInMemoryWebSocket(async ({ address, receive, send }) => {
      const client = testReactClient(address);

      const token: string | null = null;
      const fetchToken = async (args: { forceRefreshToken: boolean }) =>
        args.forceRefreshToken
          ? jwtEncode({ iat: 1234500, exp: 1244500 }, "secret")
          : token;
      const tokenFetcher = vi.fn(fetchToken);
      const onAuthChange = vi.fn();
      void client.setAuth(tokenFetcher, onAuthChange);

      expect((await receive()).type).toEqual("Connect");
      // The client authenticates after the token refetch
      expect((await receive()).type).toEqual("Authenticate");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      const querySetVersion = client.sync["remoteQuerySet"]["version"];

      // Server accepts new token
      send({
        type: "Transition",
        startVersion: querySetVersion,
        endVersion: {
          ...querySetVersion,
          identity: querySetVersion.identity + 1,
        },
        modifications: [],
      });

      await waitForExpect(() => {
        expect(onAuthChange).toHaveBeenCalledTimes(1);
      });
      await client.close();

      expect(tokenFetcher).toHaveBeenNthCalledWith(1, {
        forceRefreshToken: false,
      });
      expect(tokenFetcher).toHaveBeenNthCalledWith(2, {
        forceRefreshToken: true,
      });
      expect(onAuthChange).toHaveBeenCalledWith(true);
    });
  });

  // This is usually a misconfigured server rejecting any token
  test("Fail when tokens are always rejected without versioning", async () => {
    await testRauthenticationFails(undefined);
  });
  test("Fail when tokens are always rejected with versioning", async () => {
    await testRauthenticationFails(0);
  });

  async function testRauthenticationFails(
    authErrorBaseVersion: number | undefined,
  ) {
    await withInMemoryWebSocket(async ({ address, receive, send, close }) => {
      const client = testReactClient(address);

      const consoleSpy = vi
        .spyOn(global.console, "error")
        .mockImplementation(() => {
          // Do nothing
        });

      let token = jwtEncode({ iat: 1234500, exp: 1244500 }, "wobabloobla");
      const tokenFetcher = vi.fn(async () => token);
      const onAuthChange = vi.fn();
      client.setAuth(tokenFetcher, onAuthChange);

      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("Authenticate");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      // Token must change, otherwise client will not try to reauthenticate
      token = jwtEncode({ iat: 1234500, exp: 1244500 }, "secret");

      send({
        type: "AuthError",
        error: "bla",
        baseVersion: authErrorBaseVersion,
      });
      close();

      // The client reconnects automatically

      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("Authenticate");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      const AUTH_ERROR_MESSAGE = "bada boom";
      send({
        type: "AuthError",
        error: AUTH_ERROR_MESSAGE,
      });
      close();

      // The client reconnects automatically
      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      await waitForExpect(() => {
        expect(onAuthChange).toHaveBeenCalledTimes(1);
      });
      await client.close();

      expect(onAuthChange).toHaveBeenCalledTimes(1);
      expect(onAuthChange).toHaveBeenCalledWith(false);
      expect(consoleSpy).toHaveBeenCalledWith(
        `Failed to authenticate: "${AUTH_ERROR_MESSAGE}", check your server auth config`,
      );
    });
  }

  // This happens when "refresh token"s expired - auth provider client thinks
  // it's authed but it cannot fetch a token at all.
  test("Fail when tokens cannot be fetched", async () => {
    await withInMemoryWebSocket(async ({ address, receive }) => {
      const client = testReactClient(address);
      const tokenFetcher = vi.fn(async () => null);
      const onAuthChange = vi.fn();
      void client.setAuth(tokenFetcher, onAuthChange);

      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      await waitForExpect(() => {
        expect(onAuthChange).toHaveBeenCalledTimes(1);
      });
      await client.close();

      expect(onAuthChange).toHaveBeenCalledTimes(1);
      expect(onAuthChange).toHaveBeenCalledWith(false);
    });
  });

  test("Client is protected against token rejection race", async () => {
    await withInMemoryWebSocket(async ({ address, receive, send, close }) => {
      const client = testReactClient(address);

      const badTokenFetcher = vi.fn(async () =>
        jwtEncode({ iat: 1234500, exp: 1244500 }, "wobalooba"),
      );
      const firstOnChange = vi.fn();
      client.setAuth(badTokenFetcher, firstOnChange);

      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("Authenticate");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      const querySetVersion = client.sync["remoteQuerySet"]["version"];

      const goodTokenFetcher = vi.fn(async () =>
        jwtEncode({ iat: 1234500, exp: 1244500 }, "secret"),
      );

      const secondOnChange = vi.fn();
      client.setAuth(goodTokenFetcher, secondOnChange);

      expect((await receive()).type).toEqual("Authenticate");

      // This is the current server AuthError sequence:
      // send a message and close the connection.
      send({
        type: "AuthError",
        error: "bla",
        // Client started at 0
        baseVersion: 0,
      });
      close();

      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("Authenticate");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      send({
        type: "Transition",
        startVersion: {
          ...querySetVersion,
          // Client started at 0 after reconnect
          identity: 0,
        },
        endVersion: {
          ...querySetVersion,
          // Good token advanced to 1
          identity: 1,
        },
        modifications: [],
      });

      await waitForExpect(() => {
        expect(secondOnChange).toHaveBeenCalledTimes(1);
      });
      await client.close();

      expect(firstOnChange).toHaveBeenCalledTimes(0);
      expect(secondOnChange).toHaveBeenCalledWith(true);
    });
  });

  test("Authentication runs first", async () => {
    await withInMemoryWebSocket(async ({ address, receive, send }) => {
      const client = testReactClient(address);

      const tokenFetcher = vi.fn(async () =>
        jwtEncode({ iat: 1234500, exp: 1244500 }, "secret"),
      );
      client.setAuth(tokenFetcher);

      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("Authenticate");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      const querySetVersion = client.sync["remoteQuerySet"]["version"];

      send({
        type: "Transition",
        startVersion: querySetVersion,
        endVersion: {
          ...querySetVersion,
          // Client started at 0
          // Good token advanced to 1
          identity: 1,
        },
        modifications: [],
      });

      const tokenFetcher2 = vi.fn(async () =>
        jwtEncode({ iat: 1234550, exp: 1244550 }, "secret"),
      );
      client.setAuth(tokenFetcher2);
      client.watchQuery(anyApi.myQuery.default).onUpdate(() => {});

      // Crucially Authenticate comes first!
      expect((await receive()).type).toEqual("Authenticate");
      expect((await receive()).type).toEqual("ModifyQuerySet");
    });
  });

  test("Auth pause doesn't prevent unsubscribing from queries", async () => {
    await withInMemoryWebSocket(async ({ address, receive, send }) => {
      const client = testReactClient(address);

      const tokenFetcher = vi.fn(async () =>
        jwtEncode({ iat: 1234500, exp: 1244500 }, "secret"),
      );
      client.setAuth(tokenFetcher);

      const unsubscribe = client
        .watchQuery(anyApi.myQuery.default)
        .onUpdate(() => {});

      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("Authenticate");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      const querySetVersion = client.sync["remoteQuerySet"]["version"];

      send({
        type: "Transition",
        startVersion: querySetVersion,
        endVersion: {
          querySet: 1,
          identity: 1,
          ts: Long.fromNumber(1),
        },
        modifications: [
          {
            type: "QueryUpdated",
            queryId: 0,
            value: 42,
            logLines: [],
          },
        ],
      });

      await waitForExpect(() => {
        expect(client.sync["remoteQuerySet"]["version"].identity).toEqual(1);
      });

      let resolve: (value: string) => void;
      const tokenFetcher2 = vi.fn(
        () =>
          new Promise<string>((r) => {
            resolve = r;
          }),
      );
      // Set new auth
      client.setAuth(tokenFetcher2);

      // Unsubscribe
      unsubscribe();

      // Finish fetching the token
      resolve!(jwtEncode({ iat: 1234550, exp: 1244550 }, "secret"));

      // Crucially Authenticate comes first!
      expect(await receive()).toMatchObject({
        type: "Authenticate",
        baseVersion: 1,
      });
      expect(await receive()).toMatchObject({
        type: "ModifyQuerySet",
        modifications: [{ type: "Remove", queryId: 0 }],
        baseVersion: 1,
      });

      // Make sure we are now unpaused

      client.watchQuery(anyApi.myQuery.default).onUpdate(() => {});

      expect(await receive()).toMatchObject({
        type: "ModifyQuerySet",
        modifications: [{ type: "Add", queryId: 1 }],
        baseVersion: 2,
      });

      client.watchQuery(anyApi.myQuery.foo).onUpdate(() => {});

      expect(await receive()).toMatchObject({
        type: "ModifyQuerySet",
        modifications: [{ type: "Add", queryId: 2 }],
        baseVersion: 3,
      });
    });
  });

  test("Local state resume doesn't cause duplicate AddQuery", async () => {
    await withInMemoryWebSocket(async ({ address, receive }) => {
      const client = testReactClient(address);

      // First we subscribe
      client.watchQuery(anyApi.myQuery.default).onUpdate(() => {});

      expect((await receive()).type).toEqual("Connect");
      expect(await receive()).toMatchObject({
        type: "ModifyQuerySet",
        modifications: [{ type: "Add", queryId: 0 }],
        baseVersion: 0,
      });

      // Before the server confirms, we set auth, leading to pause
      // and unpause.
      const tokenFetcher = vi.fn(async () =>
        jwtEncode({ iat: 1234500, exp: 1244500 }, "secret"),
      );
      client.setAuth(tokenFetcher);

      // We should only send Authenticate, since we already
      // sent the Add modification
      expect(await receive()).toMatchObject({
        type: "Authenticate",
        baseVersion: 0,
      });

      // Subscribe again
      client
        .watchQuery(anyApi.myQuery.default, { foo: "bla" })
        .onUpdate(() => {});
      // Now we're sending the second query, not the first!
      expect(await receive()).toMatchObject({
        type: "ModifyQuerySet",
        modifications: [{ type: "Add", queryId: 1 }],
        baseVersion: 1,
      });
    });
  });

  test("Local state resume doesn't send both Add and Remove", async () => {
    await withInMemoryWebSocket(async ({ address, receive }) => {
      const client = testReactClient(address);

      // First we subscribe to kick off connect.
      client.watchQuery(anyApi.myQuery.default).onUpdate(() => {});

      expect((await receive()).type).toEqual("Connect");
      expect(await receive()).toMatchObject({
        type: "ModifyQuerySet",
        modifications: [{ type: "Add", queryId: 0 }],
        baseVersion: 0,
      });

      // Set slow auth, causing pause
      let resolve: (value: string) => void;
      const tokenFetcher2 = vi.fn(
        () =>
          new Promise<string>((r) => {
            resolve = r;
          }),
      );
      client.setAuth(tokenFetcher2);

      // Subscribe to second query, while paused
      const unsubscribe = client
        .watchQuery(anyApi.myQuery.default, { foo: "bla" })
        .onUpdate(() => {});

      // Subscribe third query, while paused
      client
        .watchQuery(anyApi.myQuery.default, { foo: "da" })
        .onUpdate(() => {});

      // Unsubscribe from second query, while paused
      unsubscribe();

      // Unpause ie resume
      resolve!(jwtEncode({ iat: 1234550, exp: 1244550 }, "secret"));

      // We authenticate first
      expect(await receive()).toMatchObject({
        type: "Authenticate",
        baseVersion: 0,
      });

      // We subscribe to the third query only!
      expect(await receive()).toMatchObject({
        type: "ModifyQuerySet",
        modifications: [{ type: "Add", queryId: 2 }],
        baseVersion: 1,
      });
    });
  });

  test("Local state resume refcounts", async () => {
    await withInMemoryWebSocket(async ({ address, receive }) => {
      const client = testReactClient(address);

      // First we subscribe to kick off connect.
      client.watchQuery(anyApi.myQuery.default).onUpdate(() => {});

      expect((await receive()).type).toEqual("Connect");
      expect(await receive()).toMatchObject({
        type: "ModifyQuerySet",
        modifications: [{ type: "Add", queryId: 0 }],
        baseVersion: 0,
      });

      // Set slow auth, causing pause
      let resolve: (value: string) => void;
      const tokenFetcher2 = vi.fn(
        () =>
          new Promise<string>((r) => {
            resolve = r;
          }),
      );
      client.setAuth(tokenFetcher2);

      // Subscribe to second query, while paused
      const unsubscribe = client
        .watchQuery(anyApi.myQuery.default, { foo: "bla" })
        .onUpdate(() => {});

      // Subscribe to the same query, while paused
      client
        .watchQuery(anyApi.myQuery.default, { foo: "bla" })
        .onUpdate(() => {});

      // Unsubscribe once from the second query, while paused
      unsubscribe();

      // Unpause ie resume
      resolve!(jwtEncode({ iat: 1234550, exp: 1244550 }, "secret"));

      // We authenticate first
      expect(await receive()).toMatchObject({
        type: "Authenticate",
        baseVersion: 0,
      });

      // We subscribe to the second query, because there's still one subscriber
      // on it.
      expect(await receive()).toMatchObject({
        type: "ModifyQuerySet",
        modifications: [{ type: "Add", queryId: 1 }],
        baseVersion: 1,
      });
    });
  });

  test("Local state restart doesn't send both Add and Remove", async () => {
    await withInMemoryWebSocket(async ({ address, receive }) => {
      const client = testReactClient(address);

      // Set slow auth, causing pause while connecting
      let resolve: (value: string) => void;
      const tokenFetcher2 = vi.fn(
        () =>
          new Promise<string>((r) => {
            resolve = r;
          }),
      );
      client.setAuth(tokenFetcher2);

      // Subscribe while paused and connecting
      const unsubscribe = client
        .watchQuery(anyApi.myQuery.default)
        .onUpdate(() => {});

      // Unsubscribe while paused and connecting
      unsubscribe();

      // Unpause
      resolve!(jwtEncode({ iat: 1234550, exp: 1244550 }, "secret"));

      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("Authenticate");
      expect(await receive()).toMatchObject({
        type: "ModifyQuerySet",
        baseVersion: 0,
        modifications: [],
      });
    });
  });
});
