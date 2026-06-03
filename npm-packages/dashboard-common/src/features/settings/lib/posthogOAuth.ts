const OAUTH_BASE = "https://oauth.posthog.com";
const AUTHORIZE_URL = `${OAUTH_BASE}/oauth/authorize/`;
const TOKEN_URL = `${OAUTH_BASE}/oauth/token/`;

export const POSTHOG_OAUTH_SCOPES = "openid project:read user:read";

const REGION_HOSTS = [
  { api: "https://us.posthog.com", ingest: "https://us.i.posthog.com" },
  { api: "https://eu.posthog.com", ingest: "https://eu.i.posthog.com" },
] as const;

const MESSAGE_TYPE = "convex-posthog-oauth-callback";

// PostHog can return very large project lists for big orgs; cap the number of
// pages we follow defensively rather than chase pagination forever.
const MAX_PROJECT_PAGES = 10;

export type PostHogProject = {
  name: string;
  apiKey: string;
  host: string;
};

type CallbackMessage = {
  type: typeof MESSAGE_TYPE;
  code: string | null;
  state: string | null;
  error: string | null;
  errorDescription: string | null;
};

function base64UrlEncode(bytes: Uint8Array): string {
  let str = "";
  for (let i = 0; i < bytes.length; i += 1)
    str += String.fromCharCode(bytes[i]);
  return btoa(str).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function randomString(byteLength: number): string {
  const bytes = new Uint8Array(byteLength);
  crypto.getRandomValues(bytes);
  return base64UrlEncode(bytes);
}

export async function pkceChallenge(verifier: string): Promise<string> {
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(verifier),
  );
  return base64UrlEncode(new Uint8Array(digest));
}

function clientId(origin: string): string {
  return `${origin}/api/posthog-oauth-client`;
}

function redirectUri(origin: string): string {
  return `${origin}/oauth/posthog/callback`;
}

export function buildPostHogOAuthClientMetadata(baseUrl: string): {
  client_id: string;
  client_name: string;
  client_uri: string;
  redirect_uris: string[];
  grant_types: string[];
  response_types: string[];
  token_endpoint_auth_method: string;
  application_type: string;
  scope: string;
} {
  return {
    client_id: clientId(baseUrl),
    client_name: "Convex Dashboard",
    client_uri: baseUrl,
    redirect_uris: [redirectUri(baseUrl)],
    grant_types: ["authorization_code"],
    response_types: ["code"],
    token_endpoint_auth_method: "none",
    application_type: "web",
    scope: POSTHOG_OAUTH_SCOPES,
  };
}

async function waitForCallback(
  popup: Window,
  expectedState: string,
  expectedOrigin: string,
): Promise<{ code: string }> {
  return new Promise((resolve, reject) => {
    let settled = false;
    const finish = (fn: () => void) => {
      if (settled) return;
      settled = true;
      window.removeEventListener("message", onMessage);
      window.clearInterval(closeWatch);
      fn();
    };

    const onMessage = (event: MessageEvent) => {
      if (event.origin !== expectedOrigin) return;
      const data = event.data as CallbackMessage | undefined;
      if (!data || data.type !== MESSAGE_TYPE) return;
      if (data.error) {
        finish(() =>
          reject(
            new Error(
              data.errorDescription ||
                data.error ||
                "PostHog authorization failed",
            ),
          ),
        );
        return;
      }
      if (!data.code || data.state !== expectedState) {
        finish(() => reject(new Error("Invalid PostHog OAuth response")));
        return;
      }
      const { code } = data;
      finish(() => resolve({ code }));
    };

    const closeWatch = window.setInterval(() => {
      if (popup.closed) {
        finish(() => reject(new Error("PostHog authorization was cancelled")));
      }
    }, 500);

    window.addEventListener("message", onMessage);
  });
}

async function exchangeCode(
  code: string,
  verifier: string,
  origin: string,
): Promise<string> {
  const body = new URLSearchParams({
    grant_type: "authorization_code",
    code,
    redirect_uri: redirectUri(origin),
    client_id: clientId(origin),
    code_verifier: verifier,
  });
  const res = await fetch(TOKEN_URL, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body,
  });
  if (!res.ok) {
    // eslint-disable-next-line no-console
    console.error(
      "PostHog token exchange failed",
      res.status,
      await res.text(),
    );
    throw new Error(`PostHog token exchange failed (${res.status})`);
  }
  const json = (await res.json()) as { access_token?: string };
  if (!json.access_token) {
    throw new Error("PostHog did not return an access token");
  }
  return json.access_token;
}

// Locate the user's region by hitting /users/@me/ in each region host.
// Region detection happens once: the first region that returns 2xx wins, and
// every subsequent call is made against that region only.
async function resolveUserRegion(
  accessToken: string,
  fetchImpl: typeof fetch,
): Promise<{
  api: string;
  ingest: string;
  me: { organization?: { id?: string } };
}> {
  let lastError: unknown = null;
  for (const region of REGION_HOSTS) {
    try {
      const res = await fetchImpl(`${region.api}/api/users/@me/`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      });
      if (!res.ok) {
        lastError = new Error(`${region.api}/api/users/@me/ ${res.status}`);
        continue;
      }
      const me = (await res.json()) as {
        organization?: { id?: string };
      };
      return { api: region.api, ingest: region.ingest, me };
    } catch (e) {
      lastError = e;
    }
  }
  throw lastError instanceof Error
    ? lastError
    : new Error("Unable to reach PostHog API in any region");
}

type ProjectsPage = {
  results?: Array<{ name: string; api_token: string }>;
  next?: string | null;
};

export async function fetchPostHogProjects(
  accessToken: string,
  fetchImpl: typeof fetch = fetch,
): Promise<PostHogProject[]> {
  const region = await resolveUserRegion(accessToken, fetchImpl);
  const orgId = region.me.organization?.id;
  if (!orgId) {
    throw new Error("PostHog user has no current organization");
  }

  const projects: PostHogProject[] = [];
  let url: string | null = `${region.api}/api/organizations/${orgId}/projects/`;
  for (let page = 0; page < MAX_PROJECT_PAGES && url; page += 1) {
    const res = await fetchImpl(url, {
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    if (!res.ok) {
      // eslint-disable-next-line no-console
      console.error(
        "PostHog projects fetch failed",
        res.status,
        await res.text(),
      );
      throw new Error(`PostHog projects fetch failed (${res.status})`);
    }
    const data = (await res.json()) as ProjectsPage;
    for (const p of data.results ?? []) {
      projects.push({
        name: p.name,
        apiKey: p.api_token,
        host: region.ingest,
      });
    }
    url = data.next ?? null;
  }
  return projects;
}

export async function connectPostHog(): Promise<PostHogProject[]> {
  // crypto.subtle is only available in secure contexts (HTTPS or localhost),
  // so PKCE will fail on a self-hosted dashboard served over plain HTTP.
  // Fail with a clear message instead of an opaque "undefined" error later.
  if (!window.isSecureContext || !crypto.subtle) {
    throw new Error(
      "PostHog OAuth requires the dashboard to be served over HTTPS.",
    );
  }

  const origin = window.location.origin;
  const verifier = randomString(48);
  const challenge = await pkceChallenge(verifier);
  const state = randomString(16);

  const url = new URL(AUTHORIZE_URL);
  url.searchParams.set("response_type", "code");
  url.searchParams.set("client_id", clientId(origin));
  url.searchParams.set("redirect_uri", redirectUri(origin));
  url.searchParams.set("scope", POSTHOG_OAUTH_SCOPES);
  url.searchParams.set("state", state);
  url.searchParams.set("code_challenge", challenge);
  url.searchParams.set("code_challenge_method", "S256");

  // Use a unique window name per call so that two concurrent flows (e.g. two
  // integration modals open at once) do not navigate each other's popups.
  const popup = window.open(
    url.toString(),
    `convex-posthog-oauth-${state}`,
    "popup,width=600,height=720",
  );
  if (!popup) {
    throw new Error("Popup blocked. Allow popups for this site and try again.");
  }

  const { code } = await waitForCallback(popup, state, origin);
  const accessToken = await exchangeCode(code, verifier, origin);
  const projects = await fetchPostHogProjects(accessToken);
  if (projects.length === 0) {
    throw new Error("No PostHog projects found for this account");
  }
  return projects;
}
