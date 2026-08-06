// Regression test for the "I have to refresh the page before logging in"
// bug. The orchestrator caches the `/api/orchestrator/token` session lookup
// in SWR with a 30s deduping interval (see `_app.tsx`). While signed out
// that key is cached as `null`. After a successful sign-in the app used to
// client-side navigate straight to `/`, where `IndexPage` read the *stale*
// `null` session out of the cache — within the dedupe window, so no refetch
// happened — decided the user was signed out, and bounced back to `/login`.
// A manual browser refresh cleared the in-memory cache, which is why logging
// in "worked" only after refreshing first.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SWRConfig } from "swr";
import LoginPage from "../pages/login";
import IndexPage from "../pages/index";

const mockReplace = jest.fn();
const mockSignInEmail = jest.fn();
const mockListTeams = jest.fn();

jest.mock("next/router", () => ({
  useRouter: () => ({ query: {}, replace: mockReplace }),
}));

jest.mock("../lib/auth-client", () => ({
  authClient: {
    signIn: {
      email: (...args: unknown[]) => mockSignInEmail(...args),
      social: jest.fn(),
    },
    signUp: { email: jest.fn() },
  },
}));

jest.mock("../lib/config", () => ({
  orchestratorUrl: () => "http://orchestrator.test",
}));

jest.mock("../lib/orchestratorApi", () => ({
  listTeams: (...args: unknown[]) => mockListTeams(...args),
}));

// Mirrors the SWR options `_app.tsx` installs app-wide — the 30s
// `dedupingInterval` is the part that makes the stale entry sticky.
function AppShell({ children }: { children: React.ReactNode }) {
  return (
    <SWRConfig
      value={{
        revalidateOnFocus: false,
        revalidateOnReconnect: false,
        dedupingInterval: 30_000,
        keepPreviousData: true,
        shouldRetryOnError: false,
      }}
    >
      {children}
    </SWRConfig>
  );
}

let signedIn = false;

beforeEach(() => {
  mockReplace.mockReset();
  mockSignInEmail.mockReset();
  mockListTeams.mockReset();
  signedIn = false;

  mockSignInEmail.mockImplementation(async () => {
    signedIn = true;
    return {};
  });
  mockListTeams.mockResolvedValue([{ id: 1, name: "Acme", slug: "acme" }]);

  global.fetch = jest.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url.startsWith("/api/orchestrator/token")) {
      return signedIn
        ? ({
            ok: true,
            status: 200,
            json: async () => ({
              accessToken: "pat_test",
              memberId: 1,
              teamSlug: "acme",
              role: "admin",
            }),
          } as Response)
        : ({ ok: false, status: 401 } as Response);
    }
    throw new Error(`unexpected fetch: ${url}`);
  }) as unknown as typeof fetch;
});

test("signing in lands on the team page without a manual refresh first", async () => {
  const user = userEvent.setup();

  // 1. Signed out: the index page bounces to /login and, in doing so,
  //    populates the SWR cache with a `null` session.
  const loggedOut = render(
    <AppShell>
      <IndexPage />
    </AppShell>,
  );
  await waitFor(() => expect(mockReplace).toHaveBeenCalledWith("/login"));
  loggedOut.unmount();

  // 2. The user signs in successfully on /login.
  mockReplace.mockReset();
  const login = render(
    <AppShell>
      <LoginPage />
    </AppShell>,
  );
  await user.type(screen.getByLabelText("Email"), "user@example.com");
  await user.type(screen.getByLabelText("Password"), "password123");
  await user.click(screen.getByRole("button", { name: "Sign in" }));
  await waitFor(() => expect(mockSignInEmail).toHaveBeenCalled());
  login.unmount();

  // 3. Landing back on `/` must resolve the *new* session rather than the
  //    cached signed-out one, and route to the user's team.
  mockReplace.mockReset();
  render(
    <AppShell>
      <IndexPage />
    </AppShell>,
  );

  await waitFor(() => expect(mockReplace).toHaveBeenCalledWith("/t/acme"));
  expect(mockReplace).not.toHaveBeenCalledWith("/login");
});
