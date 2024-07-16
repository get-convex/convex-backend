/**
 * @vitest-environment jsdom
 */
import { expect, vi, test, describe } from "vitest";
import jwtEncode from "jwt-encode";
import {
  nodeWebSocket,
  withInMemoryWebSocket,
} from "../browser/sync/client_node_test_helpers.js";
import { ConvexReactClient } from "./index.js";
import waitForExpect from "wait-for-expect";

const testReactClient = (address: string) =>
  new ConvexReactClient(address, {
    webSocketConstructor: nodeWebSocket,
    unsavedChangesWarning: false,
  });

// On Linux these can retry forever due to EADDRINUSE so run then sequentially.
describe.sequential("auth websocket tests", () => {
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
});
