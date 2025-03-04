import "@testing-library/jest-dom";
import { render, screen, act } from "@testing-library/react";
import mockRouter from "next-router-mock";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { useTeams } from "api/teams";
import { useProjects } from "api/projects";
import { useAuthorizeApp, useCreateTeamAccessToken } from "api/accessTokens";
import userEvent from "@testing-library/user-event";
import { AuthorizeProject } from "./AuthorizeProject";

// Mock next/router
jest.mock("next/router", () => jest.requireActual("next-router-mock"));

// Mock the LoginLayout component
jest.mock("layouts/LoginLayout", () => ({
  LoginLayout: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="login-layout">{children}</div>
  ),
}));

// Mock the LaunchDarkly hook
jest.mock("hooks/useLaunchDarkly", () => ({
  useLaunchDarkly: jest.fn(),
}));

// Mock the teams API
jest.mock("api/teams", () => ({
  useTeams: jest.fn(),
  useTeamEntitlements: jest.fn(() => ({ maxProjects: 10 })),
}));

type CreateTeamAccessTokenParams = Parameters<
  ReturnType<typeof useCreateTeamAccessToken>
>[0];
const createTeamAccessTokenMock = jest.fn(
  (_args: CreateTeamAccessTokenParams) =>
    Promise.resolve({ accessToken: "test-token" }),
);
type AuthorizeAppParams = Parameters<ReturnType<typeof useAuthorizeApp>>[0];
const authorizeAppMock = jest.fn((_args: AuthorizeAppParams) =>
  Promise.resolve({ code: "test-code" }),
);
jest.mock("api/accessTokens", () => ({
  useCreateTeamAccessToken: jest.fn(() => createTeamAccessTokenMock),
  useAuthorizeApp: jest.fn(() => authorizeAppMock),
}));

// Mock the projects API
jest.mock("api/projects", () => ({
  useProjects: jest.fn(),
}));

describe("AuthorizeProject", () => {
  const mockLaunchDarkly = {
    oauthProviderConfiguration: {
      "test-client": {
        name: "Test App",
        allowedRedirects: ["https://test-app.com/callback"],
        allowImplicitFlow: true,
      },
    },
  };

  const responseTypes = ["token", "code"];

  beforeEach(() => {
    jest.clearAllMocks();
    mockRouter.setCurrentUrl("/");
    (useLaunchDarkly as jest.Mock).mockReturnValue(mockLaunchDarkly);
    (useTeams as jest.Mock).mockReturnValue({
      selectedTeamSlug: "test-team",
      teams: [{ id: 1, name: "Test Team", slug: "test-team" }],
    });
    (useProjects as jest.Mock).mockReturnValue([
      { id: 1, name: "Test Project", slug: "test-project", isDemo: false },
    ]);
  });

  test.each(responseTypes)(
    "shows invalid redirect_uri error when redirect_uri is missing (response_type=%p)",
    (responseType) => {
      mockRouter.setCurrentUrl(
        `/?redirect_uri=https://test-app.com/callback&response_type=${responseType}`,
      );

      render(<AuthorizeProject />);
      expect(screen.getByTestId("invalid-redirect-uri")).toBeInTheDocument();
    },
  );

  test.each(responseTypes)(
    "shows invalid redirect_uri error when redirect_uri is invalid (response_type=%p)",
    (responseType) => {
      mockRouter.setCurrentUrl(
        `/?client_id=test-client&redirect_uri=https://malicious-site.com/callback&response_type=${responseType}`,
      );

      render(<AuthorizeProject />);
      expect(screen.getByTestId("invalid-redirect-uri")).toBeInTheDocument();
    },
  );

  test("redirects with error for invalid response_type", () => {
    mockRouter.setCurrentUrl(
      "/?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=magic",
    );

    render(<AuthorizeProject />);
    expect(mockRouter.asPath).toMatch(/error=unsupported_response_type/);
  });

  test("includes state parameter in error redirect if provided", () => {
    mockRouter.setCurrentUrl(
      "/?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=magic&state=test-state",
    );

    render(<AuthorizeProject />);
    expect(mockRouter.asPath).toMatch(
      /error=unsupported_response_type.*state=test-state/,
    );
  });

  test.each([null, "plain", "S12345"])(
    "redirects with error for code_challenge_method=%p",
    (challengeType) => {
      let url = `/?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=code&code_challenge=asdf`;
      if (challengeType !== null) {
        url += `&code_challenge_method=${challengeType}`;
      }
      mockRouter.setCurrentUrl(url);

      render(<AuthorizeProject />);
      expect(mockRouter.asPath).toMatch(/error=invalid_request/);
    },
  );

  test("redirects with error for invalid code_challenge", () => {
    const challenge = "iaminvalid";
    mockRouter.setCurrentUrl(
      `/?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=code&code_challenge=${challenge}&code_challenge_method=S256`,
    );

    render(<AuthorizeProject />);
    expect(mockRouter.asPath).toMatch(/error=invalid_request/);
  });

  test("redirects with error for code_challenge with implicit flow", () => {
    const challenge = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    mockRouter.setCurrentUrl(
      `/?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=token&code_challenge=${challenge}&code_challenge_method=S256`,
    );

    render(<AuthorizeProject />);
    expect(mockRouter.asPath).toMatch(/error=invalid_request/);
  });

  test.each(responseTypes)(
    "renders authorization form with valid parameters (response_type=%p)",
    (responseType) => {
      mockRouter.setCurrentUrl(
        `/?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=${responseType}`,
      );

      render(<AuthorizeProject />);

      expect(
        screen.getByText("Authorize access to your project"),
      ).toBeInTheDocument();
      expect(screen.getAllByText(/Test App/)[0]).toBeInTheDocument();
      expect(screen.getByText("Select a team")).toBeInTheDocument();
      expect(screen.getByText("Select a project")).toBeInTheDocument();
      expect(screen.getByText("Authorize Test App")).toBeInTheDocument();
    },
  );

  test("shows project creation button when under project limit", () => {
    mockRouter.setCurrentUrl(
      "/?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=token",
    );

    render(<AuthorizeProject />);
    expect(screen.getByText("Create a new project")).toBeEnabled();
  });

  test("disables project creation button when at project limit", () => {
    mockRouter.setCurrentUrl(
      "/?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=token",
    );

    // Mock reaching project limit
    (useProjects as jest.Mock).mockReturnValue(
      Array(10).fill({
        id: 1,
        name: "Test Project",
        isDemo: false,
      }),
    );

    render(<AuthorizeProject />);
    expect(screen.getByText("Create a new project")).toBeDisabled();
  });

  test.each([
    ["token", undefined],
    ["code", undefined],
    ["code", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
  ])(
    "redirects with access token on successful authorization (response_type=%p, challenge=%p)",
    async (responseType, challenge) => {
      let url = `/?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=${responseType}&state=test-state`;
      if (challenge) {
        url += `&code_challenge=${challenge}&code_challenge_method=S256`;
      }
      mockRouter.setCurrentUrl(url);

      const { getByText } = render(<AuthorizeProject />);

      // Select a project
      const projectCombobox = screen.getByLabelText("Select a project");
      await act(async () => {
        await userEvent.click(projectCombobox);
      });
      await userEvent.click(screen.getByText("Test Project"));

      // Click the authorize button and wait for async operations
      const authorizeButton = getByText("Authorize Test App");
      expect(authorizeButton).toBeEnabled();
      await act(async () => {
        await authorizeButton.click();
      });

      // Should redirect with the token
      if (responseType === "token") {
        expect(createTeamAccessTokenMock.mock.calls).toHaveLength(1);
        expect(mockRouter.asPath).toBe(
          "/callback#access_token=project%3Atest-team%3Atest-project%7Ctest-token&token_type=bearer&state=test-state",
        );
      } else {
        expect(authorizeAppMock.mock.calls).toHaveLength(1);
        expect(authorizeAppMock.mock.calls[0][0].codeChallenge).toBe(challenge);
        expect(mockRouter.asPath).toBe(
          "/callback?code=test-code&state=test-state",
        );
      }
    },
  );

  test.each(responseTypes)(
    "redirects with server_error on token creation failure (response_type=%p)",
    async (responseType) => {
      mockRouter.setCurrentUrl(
        `/?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=${responseType}&state=test-state`,
      );

      // Mock token creation failure
      const mockCreateToken = jest
        .fn()
        .mockImplementation(() =>
          Promise.reject(new Error("Failed to create token")),
        );
      if (responseType === "token") {
        (useCreateTeamAccessToken as jest.Mock).mockReturnValue(
          mockCreateToken,
        );
      } else {
        (useAuthorizeApp as jest.Mock).mockReturnValue(mockCreateToken);
      }

      const { getByText } = render(<AuthorizeProject />);

      // Select a project
      const projectCombobox = screen.getByLabelText("Select a project");
      await act(async () => {
        await userEvent.click(projectCombobox);
      });
      await userEvent.click(screen.getByText("Test Project"));
      // Click the authorize button and wait for async operations
      const authorizeButton = getByText("Authorize Test App");
      expect(authorizeButton).toBeEnabled();
      await act(async () => {
        await authorizeButton.click();
      });

      // Should redirect with error
      if (responseType === "token") {
        expect(mockRouter.asPath).toBe(
          "/callback#error=server_error&state=test-state",
        );
      } else {
        expect(mockRouter.asPath).toBe(
          "/callback?error=server_error&state=test-state",
        );
      }
    },
  );

  test.each(responseTypes)(
    "redirects with invalid_request when cancel is clicked (response_type=%p)",
    async (responseType) => {
      mockRouter.setCurrentUrl(
        `/?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=${responseType}&state=test-state`,
      );

      const { getByText } = render(<AuthorizeProject />);

      // Click the cancel button and wait for async operations
      const cancelButton = getByText("Cancel");
      await act(async () => {
        await cancelButton.click();
      });

      // Should redirect with access_denied error
      if (responseType === "token") {
        expect(mockRouter.asPath).toBe(
          "/callback#error=access_denied&state=test-state",
        );
      } else {
        expect(mockRouter.asPath).toBe(
          "/callback?error=access_denied&state=test-state",
        );
      }
    },
  );

  test("renders within LoginLayout", () => {
    mockRouter.setCurrentUrl(
      "/?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=token",
    );

    const { getByTestId } = render(<AuthorizeProject />);
    expect(getByTestId("login-layout")).toBeInTheDocument();
    expect(getByTestId("login-layout")).toContainElement(
      screen.getByText("Authorize access to your project"),
    );
  });
});
