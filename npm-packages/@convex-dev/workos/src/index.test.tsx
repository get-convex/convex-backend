import type { ReactElement } from "react";
import { LoginRequiredError } from "@workos-inc/authkit-react";
import { describe, expect, it, vi } from "vitest";
import { ConvexProviderWithAuthKit } from "./index.js";

vi.mock("react", async (importOriginal) => {
  const react = await importOriginal<typeof import("react")>();
  return {
    ...react,
    useCallback: <T,>(callback: T) => callback,
    useMemo: <T,>(factory: () => T) => factory(),
  };
});

type GetAccessToken = (options?: {
  forceRefresh?: boolean;
}) => Promise<string | null>;

type Adapter = {
  fetchAccessToken(args: {
    forceRefreshToken: boolean;
  }): Promise<string | null>;
};

function makeFetchAccessToken(getAccessToken: GetAccessToken) {
  const element = ConvexProviderWithAuthKit({
    children: null,
    client: {
      setAuth: () => {},
      clearAuth: () => {},
    },
    useAuth: () => ({
      isLoading: false,
      user: {},
      getAccessToken,
    }),
  }) as ReactElement<{ useAuth: () => Adapter }>;

  return element.props.useAuth().fetchAccessToken;
}

describe("ConvexProviderWithAuthKit", () => {
  it("returns null when AuthKit requires login", async () => {
    const getAccessToken = vi
      .fn<GetAccessToken>()
      .mockRejectedValue(new LoginRequiredError());
    const fetchAccessToken = makeFetchAccessToken(getAccessToken);

    await expect(
      fetchAccessToken({ forceRefreshToken: false }),
    ).resolves.toBeNull();
  });

  it("preserves transient token failures", async () => {
    const networkError = new TypeError("Failed to fetch");
    const getAccessToken = vi
      .fn<GetAccessToken>()
      .mockRejectedValue(networkError);
    const fetchAccessToken = makeFetchAccessToken(getAccessToken);

    await expect(fetchAccessToken({ forceRefreshToken: false })).rejects.toBe(
      networkError,
    );
  });

  it("forces AuthKit to refresh when Convex requests it", async () => {
    const getAccessToken = vi
      .fn<GetAccessToken>()
      .mockResolvedValue("fresh-token");
    const fetchAccessToken = makeFetchAccessToken(getAccessToken);

    await expect(fetchAccessToken({ forceRefreshToken: true })).resolves.toBe(
      "fresh-token",
    );
    expect(getAccessToken).toHaveBeenCalledWith({ forceRefresh: true });
  });

  it("uses AuthKit's ordinary cached-token flow by default", async () => {
    const getAccessToken = vi
      .fn<GetAccessToken>()
      .mockResolvedValue("cached-token");
    const fetchAccessToken = makeFetchAccessToken(getAccessToken);

    await expect(fetchAccessToken({ forceRefreshToken: false })).resolves.toBe(
      "cached-token",
    );
    expect(getAccessToken).toHaveBeenCalledWith();
  });
});
