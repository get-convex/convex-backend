import { webcrypto } from "crypto";
import { TextEncoder } from "util";
import {
  buildPostHogOAuthClientMetadata,
  extractOrganizationId,
  fetchPostHogProjects,
  pkceChallenge,
  POSTHOG_OAUTH_SCOPES,
} from "./posthogOAuth";

// jsdom does not expose Web Crypto or TextEncoder; polyfill from Node.
if (!globalThis.crypto?.subtle) {
  Object.defineProperty(globalThis, "crypto", {
    value: webcrypto,
    configurable: true,
  });
}
if (typeof globalThis.TextEncoder === "undefined") {
  // @ts-expect-error — Node's TextEncoder is structurally compatible.
  globalThis.TextEncoder = TextEncoder;
}

describe("pkceChallenge", () => {
  // RFC 7636 Appendix B test vector.
  it("derives the S256 challenge from the verifier", async () => {
    const verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    expect(await pkceChallenge(verifier)).toBe(
      "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
    );
  });
});

describe("extractOrganizationId", () => {
  it("reads a string organization", () => {
    expect(extractOrganizationId({ organization: "org-1" })).toBe("org-1");
  });

  it("reads organization_id flat", () => {
    expect(extractOrganizationId({ organization_id: "org-2" })).toBe("org-2");
  });

  it("reads nested organization.id", () => {
    expect(extractOrganizationId({ organization: { id: "org-3" } })).toBe(
      "org-3",
    );
  });

  it("falls back to organization.uuid", () => {
    expect(extractOrganizationId({ organization: { uuid: "org-4" } })).toBe(
      "org-4",
    );
  });

  it("returns null when nothing matches", () => {
    expect(extractOrganizationId({})).toBeNull();
    expect(extractOrganizationId(null)).toBeNull();
    expect(extractOrganizationId("oops")).toBeNull();
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
  it("falls back from US to EU on non-2xx", async () => {
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

  it("falls back from US to EU on 5xx, not just auth failures", async () => {
    const mockFetch: typeof fetch = jest.fn(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.startsWith("https://us.posthog.com")) {
        return jsonResponse(503, { detail: "down" });
      }
      if (url.startsWith("https://eu.posthog.com/api/users/@me/")) {
        return jsonResponse(200, { organization_id: "org-eu" });
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

  it("throws when neither region returns a usable response", async () => {
    const mockFetch: typeof fetch = jest.fn(async () =>
      jsonResponse(500, { detail: "boom" }),
    ) as typeof fetch;

    await expect(fetchPostHogProjects("token", mockFetch)).rejects.toThrow(
      /returned 500/,
    );
  });
});
