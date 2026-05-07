import {
  buildPostHogOAuthClientMetadata,
  fetchPostHogProjects,
  pkceChallenge,
  POSTHOG_OAUTH_SCOPES,
} from "./posthogOAuth";

describe("pkceChallenge", () => {
  // RFC 7636 Appendix B test vector.
  it("derives the S256 challenge from the verifier", async () => {
    const verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    expect(await pkceChallenge(verifier)).toBe(
      "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
    );
  });
});

describe("buildPostHogOAuthClientMetadata", () => {
  it("derives client_id and redirect_uri from baseUrl", () => {
    const meta = buildPostHogOAuthClientMetadata(
      "https://dashboard.example.com",
    );
    expect(meta.client_id).toBe(
      "https://dashboard.example.com/api/posthog-oauth-client",
    );
    expect(meta.redirect_uris).toEqual([
      "https://dashboard.example.com/oauth/posthog/callback",
    ]);
    expect(meta.scope).toBe(POSTHOG_OAUTH_SCOPES);
    expect(meta.token_endpoint_auth_method).toBe("none");
  });
});

function jsonResponse(status: number, body: unknown): Response {
  // jsdom does not expose a global Response constructor; return a minimal
  // duck-typed object covering the Response surface fetchPostHogProjects uses.
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
    text: async () => JSON.stringify(body),
  } as Response;
}

describe("fetchPostHogProjects", () => {
  it("falls back from US to EU when /users/@me/ fails on US", async () => {
    const calls: string[] = [];
    const mockFetch: typeof fetch = jest.fn(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      calls.push(url);
      if (url.startsWith("https://us.posthog.com/api/users/@me/")) {
        return jsonResponse(401, { detail: "no" });
      }
      if (url.startsWith("https://eu.posthog.com/api/users/@me/")) {
        return jsonResponse(200, { organization: { id: "org-eu" } });
      }
      if (
        url.startsWith(
          "https://eu.posthog.com/api/organizations/org-eu/projects/",
        )
      ) {
        return jsonResponse(200, {
          results: [{ name: "EU Project", api_token: "phc_eu" }],
          next: null,
        });
      }
      throw new Error(`unexpected ${url}`);
    }) as typeof fetch;

    const projects = await fetchPostHogProjects("token", mockFetch);
    expect(projects).toEqual([
      {
        name: "EU Project",
        apiKey: "phc_eu",
        host: "https://eu.i.posthog.com",
      },
    ]);
    expect(calls[0]).toContain("us.posthog.com");
    expect(calls[1]).toContain("eu.posthog.com");
  });

  it("falls back from US to EU on 5xx at /users/@me/", async () => {
    const mockFetch: typeof fetch = jest.fn(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.startsWith("https://us.posthog.com/api/users/@me/")) {
        return jsonResponse(503, { detail: "down" });
      }
      if (url.startsWith("https://eu.posthog.com/api/users/@me/")) {
        return jsonResponse(200, { organization: { id: "org-eu" } });
      }
      if (url.includes("/projects/")) {
        return jsonResponse(200, {
          results: [{ name: "P", api_token: "phc_p" }],
        });
      }
      throw new Error(`unexpected ${url}`);
    }) as typeof fetch;

    const projects = await fetchPostHogProjects("token", mockFetch);
    expect(projects).toHaveLength(1);
    expect(projects[0].host).toBe("https://eu.i.posthog.com");
  });

  it("does not bounce to EU when /projects/ fails on the resolved region", async () => {
    // Once /users/@me/ succeeds for a region, that region's token is what we
    // have — retrying the other region with a stale token would surface a
    // misleading 401.
    const calls: string[] = [];
    const mockFetch: typeof fetch = jest.fn(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      calls.push(url);
      if (url.startsWith("https://us.posthog.com/api/users/@me/")) {
        return jsonResponse(200, { organization: { id: "org-us" } });
      }
      if (url.includes("us.posthog.com/api/organizations/")) {
        return jsonResponse(500, { detail: "boom" });
      }
      throw new Error(`unexpected ${url}`);
    }) as typeof fetch;

    await expect(fetchPostHogProjects("token", mockFetch)).rejects.toThrow(
      /projects fetch failed \(500\)/,
    );
    expect(calls.every((u) => u.includes("us.posthog.com"))).toBe(true);
  });

  it("returns the US ingest host when US succeeds", async () => {
    const mockFetch: typeof fetch = jest.fn(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.endsWith("/api/users/@me/")) {
        return jsonResponse(200, { organization: { id: "org-us" } });
      }
      if (url.includes("/projects/")) {
        return jsonResponse(200, {
          results: [
            { name: "A", api_token: "phc_a" },
            { name: "B", api_token: "phc_b" },
          ],
        });
      }
      throw new Error(`unexpected ${url}`);
    }) as typeof fetch;

    const projects = await fetchPostHogProjects("token", mockFetch);
    expect(projects).toEqual([
      { name: "A", apiKey: "phc_a", host: "https://us.i.posthog.com" },
      { name: "B", apiKey: "phc_b", host: "https://us.i.posthog.com" },
    ]);
  });

  it("follows the next cursor across pages", async () => {
    const mockFetch: typeof fetch = jest.fn(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.endsWith("/api/users/@me/")) {
        return jsonResponse(200, { organization: { id: "org-us" } });
      }
      if (url === "https://us.posthog.com/api/organizations/org-us/projects/") {
        return jsonResponse(200, {
          results: [{ name: "A", api_token: "phc_a" }],
          next: "https://us.posthog.com/api/organizations/org-us/projects/?cursor=2",
        });
      }
      if (url.includes("?cursor=2")) {
        return jsonResponse(200, {
          results: [{ name: "B", api_token: "phc_b" }],
          next: null,
        });
      }
      throw new Error(`unexpected ${url}`);
    }) as typeof fetch;

    const projects = await fetchPostHogProjects("token", mockFetch);
    expect(projects.map((p) => p.name)).toEqual(["A", "B"]);
  });

  it("throws when neither region's /users/@me/ succeeds", async () => {
    const mockFetch: typeof fetch = jest.fn(async () =>
      jsonResponse(500, { detail: "boom" }),
    ) as typeof fetch;

    await expect(fetchPostHogProjects("token", mockFetch)).rejects.toThrow(
      /\/api\/users\/@me\/ 500/,
    );
  });
});
