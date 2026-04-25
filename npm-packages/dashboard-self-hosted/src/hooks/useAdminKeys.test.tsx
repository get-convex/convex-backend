import { act, renderHook, waitFor } from "@testing-library/react";
import React from "react";
import { SWRConfig } from "swr";

import { useAdminKeys } from "./useAdminKeys";

const wrapper = ({ children }: { children: React.ReactNode }) => (
  <SWRConfig value={{ provider: () => new Map(), dedupingInterval: 0 }}>
    {children}
  </SWRConfig>
);

describe("useAdminKeys", () => {
  beforeEach(() => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      text: async () =>
        JSON.stringify([
          {
            id: "a",
            name: "laptop",
            creationTime: 1,
            revokedTime: null,
            isCurrent: true,
          },
        ]),
      json: async () => [
        {
          id: "a",
          name: "laptop",
          creationTime: 1,
          revokedTime: null,
          isCurrent: true,
        },
      ],
    }) as any;
  });

  it("lists keys", async () => {
    const { result } = renderHook(
      () =>
        useAdminKeys({
          deploymentUrl: "http://localhost:3210",
          adminKey: "convex-admin-key-xyz",
        }),
      { wrapper },
    );
    await waitFor(() => expect(result.current.keys).toHaveLength(1));
    expect(result.current.keys![0].isCurrent).toBe(true);
  });

  it("creates a key and invalidates the list", async () => {
    (global.fetch as jest.Mock)
      .mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify([]),
        json: async () => [],
      })
      .mockResolvedValueOnce({
        ok: true,
        text: async () =>
          JSON.stringify({
            id: "b",
            name: "CI",
            creationTime: 2,
            adminKey: "prod:flying-fox|NEW",
          }),
        json: async () => ({
          id: "b",
          name: "CI",
          creationTime: 2,
          adminKey: "prod:flying-fox|NEW",
        }),
      })
      .mockResolvedValue({
        ok: true,
        text: async () =>
          JSON.stringify([
            {
              id: "b",
              name: "CI",
              creationTime: 2,
              revokedTime: null,
              isCurrent: false,
            },
          ]),
        json: async () => [
          {
            id: "b",
            name: "CI",
            creationTime: 2,
            revokedTime: null,
            isCurrent: false,
          },
        ],
      });

    const { result } = renderHook(
      () =>
        useAdminKeys({
          deploymentUrl: "http://localhost:3210",
          adminKey: "convex-admin-key-xyz",
        }),
      { wrapper },
    );
    await waitFor(() => expect(result.current.keys).toBeDefined());
    let created: any;
    await act(async () => {
      created = await result.current.create("CI");
    });
    expect(created.adminKey).toBe("prod:flying-fox|NEW");
    await waitFor(() => expect(result.current.keys).toHaveLength(1));
  });
});
