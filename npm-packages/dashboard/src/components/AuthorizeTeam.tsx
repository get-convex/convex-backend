import { useTeams } from "api/teams";
import Head from "next/head";
import { Button } from "@ui/Button";
import { Combobox } from "@ui/Combobox";
import { useState } from "react";
import { useFormik } from "formik";
import { useAccessToken } from "hooks/useServerSideData";
import { useRouter } from "next/router";
import { useAuthorizeApp } from "api/accessTokens";
import { LoginLayout } from "layouts/LoginLayout";
import { Sheet } from "@ui/Sheet";
import { Callout } from "@ui/Callout";
import { captureException } from "@sentry/nextjs";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { InfoCircledIcon } from "@radix-ui/react-icons";
import { Tooltip } from "@ui/Tooltip";
import {
  OAuthConfig,
  buildOAuthRedirectUrl,
  validateOAuthConfig,
} from "components/AuthorizeProject";

export function AuthorizeTeam() {
  const router = useRouter();
  const { oauthProviderConfiguration } = useLaunchDarkly();
  const [isRedirecting, setIsRedirecting] = useState(false);

  // oauth2 team token flow validation
  const oauthConfig: OAuthConfig = {
    clientId: router.query.client_id as string,
    redirectUri: router.query.redirect_uri as string,
    state: router.query.state as string | undefined,
    responseType: "code",
    codeChallenge: router.query.code_challenge as string | undefined,
    codeChallengeMethod: router.query.code_challenge_method as
      | string
      | undefined,
  };

  const { callingApplication, validatedConfig, error, errorDescription } =
    validateOAuthConfig(oauthConfig, oauthProviderConfiguration);

  const { selectedTeamSlug, teams } = useTeams();
  const team = teams?.find((t) => t.slug === selectedTeamSlug) ?? undefined;
  const [accessToken] = useAccessToken();
  const authorizeApp = useAuthorizeApp();

  const formState = useFormik({
    initialValues: {},
    onSubmit: async () => {
      if (
        isRedirecting ||
        !team ||
        !validatedConfig ||
        !validatedConfig.redirectUri
      )
        return;
      try {
        const resp = await authorizeApp({
          authnToken: accessToken,
          teamId: team.id, // Team-level auth
          clientId: validatedConfig.clientId,
          redirectUri: validatedConfig.redirectUri,
          codeChallenge: validatedConfig.codeChallenge,
          mode: "AuthorizationCode",
        });
        const redirectUrl = buildOAuthRedirectUrl(validatedConfig, {
          code: resp.code,
          state: validatedConfig?.state,
        });
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
    if (isRedirecting) return null;
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
        <title>Authorize Convex Team Access</title>
      </Head>
      <LoginLayout>
        <Sheet className="flex max-w-prose flex-col gap-4">
          <h3>Authorize access to your team</h3>
          <div className="flex flex-col gap-2">
            <p>
              Authorizing will allow{" "}
              <span className="font-semibold">{callingApplication.name}</span>{" "}
              to:
            </p>
            <ul className="list-disc pl-4">
              <li>Create new projects</li>
              <li>Create new deployments</li>
              <li>
                <div className="flex items-center gap-1">
                  Read and write data in all projects{" "}
                  <Tooltip tip="Write access to Production deployments will depend on your team-level and project-level roles.">
                    <InfoCircledIcon />
                  </Tooltip>
                </div>
              </li>
            </ul>
          </div>
          <div className="flex flex-col gap-4">
            <div className="flex flex-col gap-1">
              <Combobox
                labelHidden={false}
                options={
                  teams?.map((t) => ({ label: t.name, value: t.slug })) ?? []
                }
                label={
                  <div className="flex items-center gap-1">
                    <span>Select a team</span>
                    <Tooltip
                      tip={`${
                        callingApplication.name
                      } will only be able to operate within the selected team.`}
                    >
                      <InfoCircledIcon />
                    </Tooltip>
                  </div>
                }
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
            <div className="mt-4 ml-auto flex items-center gap-2">
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
                  !selectedTeamSlug ? "Select a team to continue" : undefined
                }
                disabled={!selectedTeamSlug || isRedirecting}
                loading={formState.isSubmitting}
              >
                Authorize
              </Button>
            </div>
          </div>
        </Sheet>
      </LoginLayout>
    </div>
  );
}
