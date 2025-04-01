import { useTeamEntitlements, useTeams } from "api/teams";
import { useProjects } from "api/projects";
import Head from "next/head";
import { Button } from "dashboard-common/elements/Button";
import { Combobox } from "dashboard-common/elements/Combobox";
import { useState } from "react";
import { useFormik } from "formik";
import { Spinner } from "dashboard-common/elements/Spinner";
import { useAccessToken } from "hooks/useServerSideData";
import { useRouter } from "next/router";
import { useCreateTeamAccessToken, useAuthorizeApp } from "api/accessTokens";
import { LoginLayout } from "layouts/LoginLayout";
import { Sheet } from "dashboard-common/elements/Sheet";
import { PlusIcon, ResetIcon } from "@radix-ui/react-icons";
import { CreateProjectForm } from "hooks/useCreateProjectModal";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import Link from "next/link";
import { Callout } from "dashboard-common/elements/Callout";
import { captureException } from "@sentry/nextjs";

export function AuthorizeProject() {
  const router = useRouter();
  const { oauthProviderConfiguration } = useLaunchDarkly();
  const [showProjectForm, setShowProjectForm] = useState(false);

  const [isRedirecting, setIsRedirecting] = useState(false);

  // oauth2 implicit grant flow validation
  const oauthConfig: OAuthConfig = {
    clientId: router.query.client_id as string,
    redirectUri: router.query.redirect_uri as string,
    state: router.query.state as string | undefined,
    responseType: router.query.response_type as string,
    codeChallenge: router.query.code_challenge as string | undefined,
    codeChallengeMethod: router.query.code_challenge_method as
      | string
      | undefined,
  };

  const { callingApplication, validatedConfig, error, errorDescription } =
    validateOAuthConfig(oauthConfig, oauthProviderConfiguration);

  const { selectedTeamSlug, teams } = useTeams();
  const team = teams?.find((t) => t.slug === selectedTeamSlug) ?? undefined;
  const {
    projects,
    selectedProjectId,
    setSelectedProjectId,
    canCreateMoreProjects,
  } = useProjectSelection(team);
  const [didCreateProject, setDidCreateProject] = useState(false);

  const [accessToken] = useAccessToken();
  const createTeamAccessToken = useCreateTeamAccessToken({
    kind: "doNotMutate",
  });
  const authorizeApp = useAuthorizeApp();

  const formState = useFormik({
    initialValues: {},
    onSubmit: async () => {
      if (isRedirecting) {
        return;
      }
      const project = projects?.find((p) => p.id === selectedProjectId)!;
      try {
        let redirectUrl;
        if (validatedConfig?.responseType === "token") {
          const token = await createTeamAccessToken({
            authnToken: accessToken,
            teamId: null,
            projectId: project.id,
            deviceName: callingApplication.name,
            appName: callingApplication.name,
            deploymentId: null, // Authorize all deployments in this project
            permissions: null, // Allow all permissions
          });
          const projectToken = `project:${team!.slug}:${project.slug}|${token.accessToken}`;
          redirectUrl = buildOAuthRedirectUrl(validatedConfig, {
            accessToken: projectToken,
            state: validatedConfig?.state,
          });
        } else if (validatedConfig?.responseType === "code") {
          const resp = await authorizeApp({
            authnToken: accessToken,
            projectId: project.id,
            clientId: validatedConfig.clientId,
            redirectUri: validatedConfig.redirectUri!,
            codeChallenge: validatedConfig.codeChallenge,
            mode: "AuthorizationCode",
          });
          redirectUrl = buildOAuthRedirectUrl(validatedConfig, {
            code: resp.code,
            state: validatedConfig?.state,
          });
        } else {
          throw new Error("unexpected response type");
        }
        setIsRedirecting(true);
        void router.replace(redirectUrl);
      } catch (e) {
        const redirectUrl = buildOAuthRedirectUrl(validatedConfig, {
          error: "server_error",
          state: validatedConfig?.state,
        });
        setIsRedirecting(true);
        void router.replace(redirectUrl);
      }
    },
  });

  if (error) {
    if (isRedirecting) {
      return null;
    }
    if (!validatedConfig?.redirectUri) {
      captureException(error);
      return (
        <div
          data-testid="invalid-redirect-uri"
          className="flex h-screen w-full items-center justify-center"
        >
          <Callout variant="error" className="max-w-prose">
            <div>
              Invalid <code>redirect_uri</code>.
              <p>
                Contact the application owner that provided this URL to you.
              </p>
            </div>
          </Callout>
        </div>
      );
    }
    const redirectUrl = buildOAuthRedirectUrl(validatedConfig, {
      error,
      errorDescription,
      state: validatedConfig?.state,
    });
    void router.replace(redirectUrl);
    setIsRedirecting(true);
    return null;
  }

  return (
    <div className="h-screen">
      <Head>
        <title>Authorize Convex Project Access</title>
      </Head>
      <LoginLayout>
        <Sheet className="flex max-w-prose flex-col gap-4">
          <h3>Authorize access to your project</h3>
          <p>
            Authorizing will give{" "}
            <span className="font-semibold">{callingApplication.name}</span>{" "}
            access to your project, mirroring your access level for the selected
            project.
          </p>
          <p>
            If your role changes, the authorized application's access will also
            change.
          </p>
          <div className="flex flex-col gap-4">
            <div className="flex flex-col gap-1">
              <Combobox
                labelHidden={false}
                options={
                  teams?.map((t) => ({
                    label: t.name,
                    value: t.slug,
                  })) ?? []
                }
                label="Select a team"
                selectedOption={selectedTeamSlug}
                setSelectedOption={(slug) => {
                  if (slug !== null) {
                    const searchParams = new URLSearchParams(
                      window.location.search,
                    );
                    searchParams.set("team", slug);
                    void router.push(`?${searchParams.toString()}`);
                  }
                }}
              />
            </div>
            {showProjectForm ? (
              <div className="flex gap-2">
                <CreateProjectForm
                  onClose={() => {
                    setShowProjectForm(false);
                  }}
                  team={team!}
                  showLabel
                  onSuccess={(project) => {
                    setSelectedProjectId(project.projectId);
                    setShowProjectForm(false);
                    setDidCreateProject(true);
                  }}
                />
                <Button
                  variant="neutral"
                  onClick={() => setShowProjectForm(false)}
                  inline
                  className="mt-7 h-fit"
                  tip="Go back to selecting a project"
                  tipSide="right"
                  icon={<ResetIcon />}
                />
              </div>
            ) : (
              <div className="flex flex-wrap items-end gap-2">
                {projects && projects.length > 0 && (
                  <div className="flex flex-col gap-1">
                    <Combobox
                      options={
                        projects.map((project) => ({
                          label: project.name,
                          value: project.id,
                        })) ?? []
                      }
                      label="Select a project"
                      labelHidden={false}
                      selectedOption={selectedProjectId}
                      setSelectedOption={setSelectedProjectId}
                      disabled={projects === null}
                    />
                  </div>
                )}
                {!didCreateProject && (
                  <div className="flex items-center gap-2">
                    {projects && projects.length > 0 && "or"}
                    <Button
                      variant="neutral"
                      onClick={() => {
                        setShowProjectForm(true);
                        setSelectedProjectId(null);
                      }}
                      icon={<PlusIcon className="h-4 w-4" />}
                      disabled={!canCreateMoreProjects}
                      tip={
                        !canCreateMoreProjects ? (
                          <>
                            You have reached the maximum number of projects for
                            your team. You may delete a project on the{" "}
                            <Link
                              href={`/t/${team?.slug}`}
                              target="_blank"
                              className="text-content-link hover:underline"
                            >
                              projects page
                            </Link>
                            .
                          </>
                        ) : undefined
                      }
                    >
                      Create a new project
                    </Button>
                  </div>
                )}
              </div>
            )}
            <div className="ml-auto mt-4 flex items-center gap-2">
              {isRedirecting && !formState.isSubmitting && <Spinner />}
              <Button
                variant="neutral"
                onClick={() => {
                  const redirectUrl = buildOAuthRedirectUrl(validatedConfig, {
                    error: "access_denied",
                    state: validatedConfig?.state,
                  });
                  setIsRedirecting(true);
                  void router.push(redirectUrl);
                }}
                disabled={isRedirecting}
              >
                Cancel
              </Button>
              <Button
                variant="primary"
                onClick={() => formState.handleSubmit()}
                tip={
                  !selectedProjectId
                    ? "Select or create a project to continue"
                    : undefined
                }
                disabled={
                  formState.isSubmitting || !selectedProjectId || isRedirecting
                }
              >
                {formState.isSubmitting
                  ? "Authorizing"
                  : `Authorize ${callingApplication.name}`}
              </Button>
            </div>
          </div>
        </Sheet>
      </LoginLayout>
    </div>
  );
}

// from RFC 6749 section 4.1.2.1
type OAuthError =
  | "invalid_request"
  | "unauthorized_client"
  | "access_denied"
  | "unsupported_response_type"
  | "invalid_scope"
  | "server_error"
  | "temporarily_unavailable";

interface OAuthConfig {
  clientId: string;
  redirectUri: string;
  state?: string;
  responseType: string;
  codeChallenge?: string;
  codeChallengeMethod?: string;
}

interface ValidatedOAuthConfig {
  clientId: string;
  redirectUri?: string; // Optional since it may be invalid
  state?: string;
  responseType?: "token" | "code";
  codeChallenge?: string;
}

function validateOAuthConfig(
  config: OAuthConfig,
  oauthProviderConfiguration: Record<
    string,
    {
      name: string;
      allowedRedirects: string[];
      allowImplicitFlow?: boolean;
      allowedRedirectsAnySubdomain?: string[];
    }
  >,
): {
  callingApplication: { name: string; allowedRedirects: string[] };
  validatedConfig?: ValidatedOAuthConfig;
  error?: OAuthError;
  errorDescription?: string;
} {
  const callingApplication = oauthProviderConfiguration[config.clientId];

  if (!callingApplication) {
    return { callingApplication, error: "invalid_request" };
  }

  const validatedConfig: ValidatedOAuthConfig = {
    clientId: config.clientId,
    state: config.state,
  };

  if (!config.redirectUri) {
    return {
      callingApplication,
      validatedConfig,
      error: "invalid_request",
    };
  }

  if (!redirectUriAllowed(callingApplication, config.redirectUri)) {
    // Don't include the invalid redirectUri in the validated config
    return {
      callingApplication,
      validatedConfig,
      error: "invalid_request",
      errorDescription: "unknown redirect_uri",
    };
  }
  validatedConfig.redirectUri = config.redirectUri;

  if (
    !(
      (config.responseType === "token" &&
        callingApplication.allowImplicitFlow) ||
      config.responseType === "code"
    )
  ) {
    return {
      callingApplication,
      validatedConfig,
      error: "unsupported_response_type",
    };
  }
  validatedConfig.responseType = config.responseType;

  if (config.codeChallenge) {
    if (config.codeChallengeMethod !== "S256") {
      return {
        callingApplication,
        validatedConfig,
        error: "invalid_request",
        errorDescription: "unsupported code_challenge_method",
      };
    }
    if (validatedConfig.responseType !== "code") {
      return {
        callingApplication,
        validatedConfig,
        error: "invalid_request",
        errorDescription: "code_challenge must be used with response_type=code",
      };
    }
    if (config.codeChallenge.length !== 43) {
      return {
        callingApplication,
        validatedConfig,
        error: "invalid_request",
        errorDescription: "code_challenge is the wrong length for S256",
      };
    }
    validatedConfig.codeChallenge = config.codeChallenge;
  }

  return {
    callingApplication,
    validatedConfig,
  };
}

function redirectUriAllowed(
  callingApplication: {
    allowedRedirectsAnySubdomain?: string[];
    allowedRedirects: string[];
  },
  redirectUri: string,
): boolean {
  if (callingApplication.allowedRedirects.includes(redirectUri)) {
    return true;
  }

  return (callingApplication.allowedRedirectsAnySubdomain || []).some(
    (allowedPattern) => {
      // https://    a.pages.dev/foo allows both
      // https://  b.a.pages.dev/foo and
      // https://c.b.a.pages.dev/foo.
      let allowedUrl: URL;
      let redirectUrl: URL;
      try {
        allowedUrl = new URL(allowedPattern);
        redirectUrl = new URL(redirectUri);
      } catch (e) {
        captureException(e);
        return false;
      }

      if (
        redirectUrl.hostname !== allowedUrl.hostname &&
        !redirectUrl.hostname.endsWith(`.${allowedUrl.hostname}`)
      ) {
        return false;
      }

      return (
        redirectUrl.protocol === allowedUrl.protocol &&
        redirectUrl.port === allowedUrl.port &&
        redirectUrl.pathname === allowedUrl.pathname &&
        redirectUrl.search === allowedUrl.search
      );
    },
  );
}

function buildOAuthRedirectUrl(
  validatedConfig: ValidatedOAuthConfig | undefined,
  params: {
    error?: OAuthError;
    errorDescription?: string;
    accessToken?: string;
    code?: string;
    state?: string;
  },
): string {
  const redirectUri = validatedConfig?.redirectUri;

  // If no valid redirectUri was provided, redirect to a safe error page
  if (!redirectUri) {
    throw new Error("redirectUri is missing");
  }

  try {
    const url = new URL(redirectUri);
    const hashParams: string[] = [];

    if (params.error) {
      hashParams.push(`error=${encodeURIComponent(params.error)}`);
      if (params.errorDescription) {
        hashParams.push(
          `error_description=${encodeURIComponent(params.errorDescription)}`,
        );
      }
    } else if (validatedConfig.responseType === "token" && params.accessToken) {
      hashParams.push(`access_token=${encodeURIComponent(params.accessToken)}`);
      hashParams.push("token_type=bearer");
    } else if (validatedConfig.responseType === "code" && params.code) {
      hashParams.push(`code=${encodeURIComponent(params.code)}`);
    }

    if (params.state) {
      hashParams.push(`state=${encodeURIComponent(params.state)}`);
    }

    const responseParams = hashParams.join("&");
    if (validatedConfig.responseType === "token") {
      // implicit flow returns a token in the hash part of the URL
      url.hash = responseParams;
    } else {
      // authorization code flow returns a code in the query parameters
      url.search = responseParams;
    }
    return url.toString();
  } catch (e) {
    throw new Error("redirectUri is invalid");
  }
}

function useProjectSelection(team?: { id: number }) {
  const projects = useProjects(team?.id, 30000);
  const [selectedProjectId, setSelectedProjectId] = useState<number | null>(
    null,
  );
  const entitlements = useTeamEntitlements(team?.id);
  const canCreateMoreProjects =
    projects && entitlements && projects.length < entitlements.maxProjects;

  return {
    projects,
    selectedProjectId,
    setSelectedProjectId,
    canCreateMoreProjects,
  };
}
