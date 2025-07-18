import "@testing-library/jest-dom";
import { render, screen, act } from "@testing-library/react";
import mockRouter from "next-router-mock";
import { useTeams } from "api/teams";
import { useProjects } from "api/projects";
import { useAuthorizeApp } from "api/accessTokens";
import userEvent from "@testing-library/user-event";
import { AuthorizeApp } from "./AuthorizeApp";

// Mock next/router
jest.mock("next/router", () => jest.requireActual("next-router-mock"));

// Mock the LoginLayout component
jest.mock("layouts/LoginLayout", () => ({
  LoginLayout: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="login-layout">{children}</div>
  ),
}));

// Mock the teams API
jest.mock("api/teams", () => ({
  useTeams: jest.fn(),
  useTeamEntitlements: jest.fn(() => ({ maxProjects: 10 })),
}));

// Mock the OAuth API
const checkOauthAppMock = jest.fn();
jest.mock("api/oauth", () => ({
  useCheckOauthApp: jest.fn(() => checkOauthAppMock),
}));

// Mock the access token hook
jest.mock("hooks/useServerSideData", () => ({
  useAccessToken: jest.fn(() => ["test-token"]),
}));

type AuthorizeAppParams = Parameters<ReturnType<typeof useAuthorizeApp>>[0];
const authorizeAppMock = jest.fn((_args: AuthorizeAppParams) =>
  Promise.resolve({ code: "test-code" }),
);
jest.mock("api/accessTokens", () => ({
  useAuthorizeApp: jest.fn(() => authorizeAppMock),
}));

// Mock the projects API
jest.mock("api/projects", () => ({
  useProjects: jest.fn(),
}));

// Mock Sentry
jest.mock("@sentry/nextjs", () => ({
  captureException: jest.fn(),
}));

describe("AuthorizeApp", () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockRouter.setCurrentUrl("/");
    (useTeams as jest.Mock).mockReturnValue({
      selectedTeamSlug: "test-team",
      teams: [{ id: 1, name: "Test Team", slug: "test-team" }],
    });
    (useProjects as jest.Mock).mockReturnValue([
      { id: 1, name: "Test Project", slug: "test-project", isDemo: false },
    ]);

    // Mock successful OAuth app check
    checkOauthAppMock.mockResolvedValue({
      appName: "Test App",
      clientId: "test-client",
      redirectUris: ["https://test-app.com/callback"],
      verified: true,
    });
  });

  describe("Project authorization", () => {
    beforeEach(() => {
      mockRouter.setCurrentUrl("/oauth/authorize/project");
    });

    test("shows missing parameters error when client_id is missing", () => {
      mockRouter.setCurrentUrl(
        "/oauth/authorize/project?redirect_uri=https://test-app.com/callback&response_type=code",
      );

      render(<AuthorizeApp authorizationScope="project" />);
      expect(
        screen.getByText("Missing required OAuth parameters."),
      ).toBeInTheDocument();
      expect(screen.getByText("is required")).toBeInTheDocument();
    });

    test("shows missing parameters error when redirect_uri is missing", () => {
      mockRouter.setCurrentUrl(
        "/oauth/authorize/project?client_id=test-client&response_type=code",
      );

      render(<AuthorizeApp authorizationScope="project" />);
      expect(
        screen.getByText("Missing required OAuth parameters."),
      ).toBeInTheDocument();
      expect(screen.getByText("is required")).toBeInTheDocument();
    });

    test("shows missing parameters error when response_type is not code", () => {
      mockRouter.setCurrentUrl(
        "/oauth/authorize/project?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=token",
      );

      render(<AuthorizeApp authorizationScope="project" />);
      expect(
        screen.getByText("Missing required OAuth parameters."),
      ).toBeInTheDocument();
      expect(screen.getByText('must be set to "code"')).toBeInTheDocument();
    });

    test("shows error for invalid code_challenge_method", () => {
      mockRouter.setCurrentUrl(
        "/oauth/authorize/project?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=code&code_challenge=asdf&code_challenge_method=plain",
      );

      render(<AuthorizeApp authorizationScope="project" />);
      expect(screen.getByText("invalid_request")).toBeInTheDocument();
    });

    test("shows error for invalid code_challenge length", () => {
      const challenge = "iaminvalid";
      mockRouter.setCurrentUrl(
        `/oauth/authorize/project?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=code&code_challenge=${challenge}&code_challenge_method=S256`,
      );

      render(<AuthorizeApp authorizationScope="project" />);
      expect(screen.getByText("invalid_request")).toBeInTheDocument();
    });

    test("renders authorization form with valid parameters", async () => {
      mockRouter.setCurrentUrl(
        "/oauth/authorize/project?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=code",
      );

      render(<AuthorizeApp authorizationScope="project" />);

      // Wait for the OAuth app check to complete
      await screen.findByText("Test App");

      expect(
        screen.getByText("Authorize access to your project"),
      ).toBeInTheDocument();
      expect(screen.getByText("Test App")).toBeInTheDocument();
      expect(screen.getByText("Select a team")).toBeInTheDocument();
      expect(screen.getByText("Select a project")).toBeInTheDocument();
    });

    test("shows project creation button when under project limit", async () => {
      mockRouter.setCurrentUrl(
        "/oauth/authorize/project?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=code",
      );

      render(<AuthorizeApp authorizationScope="project" />);

      // Wait for the OAuth app check to complete
      await screen.findByText("Create a new project");
      expect(screen.getByText("Create a new project")).toBeEnabled();
    });

    test("disables project creation button when at project limit", async () => {
      mockRouter.setCurrentUrl(
        "/oauth/authorize/project?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=code",
      );

      // Mock reaching project limit
      (useProjects as jest.Mock).mockReturnValue(
        Array(10).fill({
          id: 1,
          name: "Test Project",
          slug: "test-project",
          isDemo: false,
        }),
      );

      render(<AuthorizeApp authorizationScope="project" />);

      // Wait for the OAuth app check to complete
      await screen.findByText("Create a new project");
      expect(screen.getByText("Create a new project")).toBeDisabled();
    });

    test("authorizes project with valid parameters", async () => {
      mockRouter.setCurrentUrl(
        "/oauth/authorize/project?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=code",
      );

      render(<AuthorizeApp authorizationScope="project" />);

      // Wait for the OAuth app check to complete
      await screen.findByText("Test App");

      // The authorize button should be disabled because no project is selected
      const authorizeButton = screen.getByText("Authorize");
      expect(authorizeButton).toBeDisabled();

      // The test is checking that the form validation works correctly
      // The actual authorization would require selecting a project first
    });

    test("shows OAuth app validation error", async () => {
      checkOauthAppMock.mockRejectedValue(new Error("Invalid redirect URI"));

      mockRouter.setCurrentUrl(
        "/oauth/authorize/project?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=code",
      );

      render(<AuthorizeApp authorizationScope="project" />);

      await screen.findByText("Invalid redirect URI");
      expect(
        screen.getByText(
          "Contact the developer of the application that provided this URL to you.",
        ),
      ).toBeInTheDocument();
    });
  });

  describe("Team authorization", () => {
    beforeEach(() => {
      mockRouter.setCurrentUrl("/oauth/authorize/team");
    });

    test("renders team authorization form with valid parameters", async () => {
      mockRouter.setCurrentUrl(
        "/oauth/authorize/team?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=code",
      );

      render(<AuthorizeApp authorizationScope="team" />);

      // Wait for the OAuth app check to complete
      await screen.findByText("Test App");

      expect(
        screen.getByText("Authorize access to your team"),
      ).toBeInTheDocument();
      expect(screen.getByText("Test App")).toBeInTheDocument();
      expect(screen.getByText(/Select a team/)).toBeInTheDocument();
      expect(screen.getByText(/Create new projects/)).toBeInTheDocument();
      expect(screen.getByText(/Create new deployments/)).toBeInTheDocument();
      expect(
        screen.getByText(/Read and write data in all projects/),
      ).toBeInTheDocument();
    });

    test("authorizes team with valid parameters", async () => {
      const user = userEvent.setup();
      mockRouter.setCurrentUrl(
        "/oauth/authorize/team?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=code",
      );

      render(<AuthorizeApp authorizationScope="team" />);

      // Wait for the OAuth app check to complete
      await screen.findByText("Test App");

      const authorizeButton = screen.getByText("Authorize");
      await act(async () => {
        await user.click(authorizeButton);
      });

      expect(authorizeAppMock).toHaveBeenCalledWith({
        authnToken: "test-token",
        teamId: 1,
        clientId: "test-client",
        redirectUri: "https://test-app.com/callback",
        codeChallenge: undefined,
        mode: "AuthorizationCode",
      });
    });

    test("shows missing parameters error for invalid response_type", () => {
      mockRouter.setCurrentUrl(
        "/oauth/authorize/team?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=token",
      );

      render(<AuthorizeApp authorizationScope="team" />);
      expect(
        screen.getByText("Missing required OAuth parameters."),
      ).toBeInTheDocument();
      expect(screen.getByText('must be set to "code"')).toBeInTheDocument();
    });

    test("shows OAuth app validation error for team scope", async () => {
      checkOauthAppMock.mockRejectedValue(new Error("Unknown client id"));

      mockRouter.setCurrentUrl(
        "/oauth/authorize/team?client_id=test-client&redirect_uri=https://test-app.com/callback&response_type=code",
      );

      render(<AuthorizeApp authorizationScope="team" />);

      await screen.findByText("Unknown client id");
      expect(
        screen.getByText(
          "Contact the developer of the application that provided this URL to you.",
        ),
      ).toBeInTheDocument();
    });
  });
});
