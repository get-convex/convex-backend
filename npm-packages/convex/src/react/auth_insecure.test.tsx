/**
 * @vitest-environment custom-vitest-environment.ts
 */
import { expect, vi, test, describe, beforeEach } from "vitest";
import jwtEncode from "jwt-encode";
import {
  nodeWebSocket,
  withInMemoryWebSocket,
} from "../browser/sync/client_node_test_helpers.js";
import { ConvexReactClient, ConvexReactClientOptions } from "./index.js";
import waitForExpect from "wait-for-expect";
import { anyApi } from "../server/index.js";
import { Long } from "../browser/long.js";
import {
  AuthError,
  ClientMessage,
  ServerMessage,
} from "../browser/sync/protocol.js";

const testReactClient = (address: string, options?: ConvexReactClientOptions) =>
  new ConvexReactClient(address, {
    webSocketConstructor: nodeWebSocket,
    unsavedChangesWarning: false,
    ...options,
  });

describe("setAuthInsecure functionality", () => {
  test("setAuthInsecure establishes plaintext authentication", async () => {
    await withInMemoryWebSocket(async ({ address, receive, send }) => {
      const client = testReactClient(address);

      const plaintextToken = "my-plaintext-test-token-12345";
      const tokenFetcher = vi.fn(async () => plaintextToken);
      const onAuthChange = vi.fn();

      client.setAuthInsecure(tokenFetcher, onAuthChange);

      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("Authenticate");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      const querySetVersion = client.sync["remoteQuerySet"]["version"];

      send({
        type: "Transition",
        startVersion: querySetVersion,
        endVersion: {
          ...querySetVersion,
          identity: 1,
        },
        modifications: [],
      });

      await waitForExpect(() => {
        expect(onAuthChange).toHaveBeenCalledTimes(1);
      });
      await client.close();

      expect(tokenFetcher).toHaveBeenCalledWith({ forceRefreshToken: false });
      expect(onAuthChange).toHaveBeenCalledWith(true);
    });
  });

  test("switching from setAuth to setAuthInsecure", async () => {
    await withInMemoryWebSocket(async ({ address, receive, send }) => {
      const client = testReactClient(address);

      // First set regular JWT auth
      const jwtToken = jwtEncode(
        { iat: 1234500, exp: 1244500, name: "User" },
        "secret",
      );
      const jwtTokenFetcher = vi.fn(async () => jwtToken);
      const jwtOnAuthChange = vi.fn();

      client.setAuth(jwtTokenFetcher, jwtOnAuthChange);

      expect((await receive()).type).toEqual("Connect");
      assertAuthenticateMessage(await receive(), {
        baseVersion: 0,
        token: jwtToken,
        tokenType: "User",
      });
      expect((await receive()).type).toEqual("ModifyQuerySet");

      const querySetVersion = client.sync["remoteQuerySet"]["version"];

      send({
        type: "Transition",
        startVersion: querySetVersion,
        endVersion: {
          ...querySetVersion,
          identity: 1,
        },
        modifications: [],
      });

      await waitForExpect(() => {
        expect(jwtOnAuthChange).toHaveBeenCalledTimes(1);
      });

      // Now switch to plaintext auth
      const plaintextToken = "plaintext-token-67890";
      const plaintextTokenFetcher = vi.fn(async () => plaintextToken);
      const plaintextOnAuthChange = vi.fn();

      client.setAuthInsecure(plaintextTokenFetcher, plaintextOnAuthChange);

      // Should receive Authenticate message with plaintext token
      assertAuthenticateMessage(await receive(), {
        baseVersion: 1,
        token: plaintextToken,
        tokenType: "PlaintextUser",
      });

      send({
        type: "Transition",
        startVersion: {
          ...querySetVersion,
          identity: 1,
        },
        endVersion: {
          ...querySetVersion,
          identity: 2,
        },
        modifications: [],
      });

      await waitForExpect(() => {
        expect(plaintextOnAuthChange).toHaveBeenCalledTimes(1);
      });
      await client.close();

      expect(plaintextTokenFetcher).toHaveBeenCalledWith({
        forceRefreshToken: false,
      });
      expect(plaintextOnAuthChange).toHaveBeenCalledWith(true);
    });
  });

  test("clearAuth after setAuthInsecure", async () => {
    await withInMemoryWebSocket(async ({ address, receive, send }) => {
      const client = testReactClient(address);

      const plaintextToken = "test-plaintext-token";
      const tokenFetcher = vi.fn(async () => plaintextToken);
      const onAuthChange = vi.fn();

      client.setAuthInsecure(tokenFetcher, onAuthChange);

      expect((await receive()).type).toEqual("Connect");
      assertAuthenticateMessage(await receive(), {
        baseVersion: 0,
        token: plaintextToken,
        tokenType: "PlaintextUser",
      });
      expect((await receive()).type).toEqual("ModifyQuerySet");

      const querySetVersion = client.sync["remoteQuerySet"]["version"];

      send({
        type: "Transition",
        startVersion: querySetVersion,
        endVersion: {
          ...querySetVersion,
          identity: 1,
        },
        modifications: [],
      });

      await waitForExpect(() => {
        expect(onAuthChange).toHaveBeenCalledTimes(1);
      });

      // this doesn't result in onAuthChange being called.
      client.clearAuth();

      await waitForExpect(() => {
        expect(onAuthChange).toHaveBeenCalledTimes(1);
      });

      await client.close();

      expect(onAuthChange).toHaveBeenNthCalledWith(1, true);
    });
  });

  test("setAuthInsecure handles token refresh", async () => {
    await withInMemoryWebSocket(async ({ address, receive, send, close }) => {
      const client = testReactClient(address);

      let tokenCount = 0;
      const tokenFetcher = vi.fn(async () => `plaintext-token-${++tokenCount}`);
      const onAuthChange = vi.fn();

      client.setAuthInsecure(tokenFetcher, onAuthChange);

      await assertReconnectWithAuth(receive, {
        baseVersion: 0,
        token: "plaintext-token-1",
        tokenType: "PlaintextUser",
      });

      // Simulate auth error requiring token refresh
      await simulateAuthError({
        send,
        close,
        authError: {
          type: "AuthError",
          error: "plaintext token expired",
          baseVersion: 0,
          authUpdateAttempted: true,
        },
      });

      // The client reconnects with a new token
      await assertReconnectWithAuth(receive, {
        baseVersion: 0,
        token: "plaintext-token-2",
        tokenType: "PlaintextUser",
      });

      const querySetVersion = client.sync["remoteQuerySet"]["version"];

      send({
        type: "Transition",
        startVersion: querySetVersion,
        endVersion: {
          ...querySetVersion,
          identity: 1,
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

  test("setAuthInsecure fails when tokens cannot be fetched", async () => {
    await withInMemoryWebSocket(async ({ address, receive }) => {
      const client = testReactClient(address);
      const tokenFetcher = vi.fn(async () => null);
      const onAuthChange = vi.fn();

      client.setAuthInsecure(tokenFetcher, onAuthChange);

      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      await waitForExpect(() => {
        expect(onAuthChange).toHaveBeenCalledTimes(1);
      });
      await client.close();

      expect(onAuthChange).toHaveBeenCalledWith(false);
    });
  });
});

function assertAuthenticateMessage(
  message: ClientMessage,
  expected: {
    baseVersion: number;
    token: string;
    tokenType: "User" | "PlaintextUser";
  },
) {
  expect(message.type).toEqual("Authenticate");
  if (message.type !== "Authenticate") {
    throw new Error("Expected an Authenticate message");
  }
  expect(message.baseVersion).toEqual(expected.baseVersion);
  expect(message.tokenType).toEqual(expected.tokenType);
  if (message.tokenType === "User" || message.tokenType === "PlaintextUser") {
    expect(message.value).toEqual(expected.token);
  }
}

async function assertReconnectWithAuth(
  receive: () => Promise<ClientMessage>,
  expectedAuth: {
    baseVersion: number;
    token: string;
    tokenType: "User" | "PlaintextUser";
  },
) {
  expect((await receive()).type).toEqual("Connect");
  assertAuthenticateMessage(await receive(), expectedAuth);
  expect((await receive()).type).toEqual("ModifyQuerySet");
}

async function simulateAuthError(args: {
  send: (message: ServerMessage) => void;
  close: () => void;
  authError: AuthError;
}) {
  args.send({
    type: "AuthError",
    error: args.authError.error,
    baseVersion: args.authError.baseVersion,
    authUpdateAttempted: args.authError.authUpdateAttempted,
  });
  args.close();
}
