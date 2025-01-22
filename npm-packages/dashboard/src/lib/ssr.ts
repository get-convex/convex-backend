import type { GetServerSideProps, GetServerSidePropsContext } from "next";
import { GetAccessTokenResult } from "@auth0/nextjs-auth0";
import { auth0, withPageAuthRequired } from "server/auth0";
import groupBy from "lodash/groupBy";
import { DeploymentResponse, Team, ProjectDetails } from "generatedApi";
import fetchRetryFactory from "fetch-retry";
import { getGoogleAnalyticsClientId } from "hooks/fetching";

const getProps: GetServerSideProps<{
  [key: string]: any;
}> = async ({ req, res, query, resolvedUrl }) => {
  const googleAnalyticsId = req.headers.cookie
    ? getGoogleAnalyticsClientId(req.headers.cookie)
    : "";

  const isFirstServerCall = req?.url?.indexOf("/_next/data/") === -1;
  const shouldRedirectToDeploymentPage = resolvedUrl.startsWith("/d/");
  const shouldRedirectToProjectPage = resolvedUrl.startsWith("/p/");

  // If this is not the first time we're loading props, we can return early
  // and not re-fetch additional data.
  if (
    !isFirstServerCall &&
    !shouldRedirectToDeploymentPage &&
    !shouldRedirectToProjectPage
  ) {
    return { props: {} };
  }

  let token: GetAccessTokenResult | undefined;
  try {
    token = await auth0().getAccessToken(req, res);
  } catch (error) {
    console.error("Couldn't fetch auth token", error);
    // If we can't get the token, we should try to login again
    res.writeHead(307, { Location: `/api/auth/login?returnTo=${req.url}` });
    res.end();
    return { props: {} };
  }

  if (!token) {
    return { props: {} };
  }

  if (process.env.DISABLE_BIG_BRAIN_SSR) {
    return {
      props: { accessToken: token.accessToken },
    };
  }

  try {
    const resp = await retryingFetch(
      `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard/member_data`,
      {
        headers: {
          authorization: `Bearer ${token.accessToken}`,
          "Google-Analytics-Client-Id": googleAnalyticsId,
        },
      },
    );
    if (!resp.ok) {
      try {
        const error: { message: string; code: string } = await resp.json();
        if (resp.status === 400) {
          return {
            props: {
              error,
            },
          };
        }

        console.error("Couldn't fetch member data", error);
        return {
          props: {
            accessToken: token.accessToken,
            error,
          },
        };
      } catch (e) {
        console.error("Couldn't fetch member data", e);
        return {
          props: {
            accessToken: token.accessToken,
            error: {
              message: "Failed to connect to Convex dashboard",
              code: "FailedToConnect",
            },
          },
        };
      }
    }

    const {
      teams,
      projects,
      deployments,
      optInsToAccept,
    }: {
      teams: Team[];
      projects: ProjectDetails[];
      deployments: DeploymentResponse[];
      optInsToAccept?: {
        optIn: string;
        message: string;
      }[];
    } = await resp.json();
    const { team, project, deploymentName } = query;
    if (
      (team && !teams.find((t: Team) => t.slug === team.toString())) ||
      (project &&
        !projects.find((p: ProjectDetails) => p.slug === project.toString()))
    ) {
      // You're looking for a page that doesn't exist!
      return pageNotFound(res);
    }

    if (shouldRedirectToProjectPage && project !== undefined) {
      return redirectToProjectPage(
        resolvedUrl,
        res,
        project as string,
        teams,
        projects,
      );
    }

    if (shouldRedirectToDeploymentPage && deploymentName !== undefined) {
      return redirectToDeploymentPage(
        resolvedUrl,
        res,
        deploymentName as string,
        teams,
        projects,
        deployments,
      );
    }

    const projectsByTeam = groupBy(projects, (p: ProjectDetails) => p.teamId);

    const initialProjects = Object.fromEntries(
      teams.map(({ id: teamId }) => [
        `/teams/${teamId}/projects`,
        projectsByTeam[teamId] ?? [],
      ]),
    );

    const deploymentsByProject = groupBy(
      deployments,
      (d: DeploymentResponse) => d.projectId,
    );
    const initialDeployments = Object.fromEntries(
      projects.map(({ id: projectId }) => [
        `/projects/${projectId}/instances`,
        deploymentsByProject[projectId] ?? null,
      ]),
    );

    const initialData: Record<string, object> = {
      "/teams": teams,
      ...initialProjects,
      ...initialDeployments,
    };

    if (optInsToAccept !== undefined) {
      initialData["/optins"] = { optInsToAccept };
    }

    return {
      props: { accessToken: token.accessToken, initialData },
      redirect:
        optInsToAccept && optInsToAccept.length > 0 && resolvedUrl !== "/accept"
          ? {
              destination: "/accept",
              permanent: false,
              query: resolvedUrl ? { from: resolvedUrl } : undefined,
            }
          : undefined,
    };
  } catch (e: any) {
    return {
      props: {
        accessToken: token.accessToken,
        error: { message: e.message, code: "FailedToConnect" },
      },
    };
  }
};

export const getServerSideProps = withPageAuthRequired({
  getServerSideProps: getProps,
});

function redirectToProjectPage(
  resolvedUrl: string,
  res: GetServerSidePropsContext["res"],
  projectSlug: string,
  teams: Team[],
  projects: ProjectDetails[],
) {
  const project = projects.find((p: ProjectDetails) => p.slug === projectSlug);
  const owningTeam = teams.find((t: Team) => t.id === project?.teamId);

  if (owningTeam === undefined || project === undefined) {
    return pageNotFound(res);
  }
  const remainingPath = resolvedUrl.slice(`/p/${projectSlug}`.length);
  return {
    redirect: {
      permanent: false,
      destination: `/t/${owningTeam.slug}/${project.slug}${remainingPath}`,
    },
  };
}

function redirectToDeploymentPage(
  resolvedUrl: string,
  res: GetServerSidePropsContext["res"],
  deploymentName: string,
  teams: Team[],
  projects: ProjectDetails[],
  deployments: DeploymentResponse[],
) {
  const deployment = deployments.find(
    (d: DeploymentResponse) => d.name === deploymentName,
  );

  const owningProject = projects.find(
    (p: ProjectDetails) => p.id === deployment?.projectId,
  );
  const owningTeam = teams.find((t: Team) => t.id === owningProject?.teamId);
  if (
    owningTeam === undefined ||
    owningProject === undefined ||
    deployment === undefined
  ) {
    return pageNotFound(res);
  }
  const remainingPath = resolvedUrl.slice(`/d/${deploymentName}`.length);
  return {
    redirect: {
      permanent: false,
      destination: `/t/${owningTeam.slug}/${owningProject.slug}/${deploymentName}${remainingPath}`,
    },
  };
}

function pageNotFound(res: GetServerSidePropsContext["res"]) {
  res.writeHead(307, { Location: "/404" });
  res.end();
  return { props: {} };
}

export const retryingFetch = fetchRetryFactory(fetch, {
  retries: 4,
  retryDelay: (attempt: number, _error: any, _response: any) => {
    // immediate, 1s delay, 2s delay, 4s delay, etc.
    const delay = attempt === 0 ? 1 : 2 ** (attempt - 1) * 1000;
    const randomSum = delay * 0.2 * Math.random();
    return delay + randomSum;
  },
  retryOn(_attempt: number, error: Error | null, _response: Response | null) {
    // Retry on network errors.
    if (error) {
      // TODO filter out all SSL errors
      // https://github.com/nodejs/node/blob/8a41d9b636be86350cd32847c3f89d327c4f6ff7/src/crypto/crypto_common.cc#L218-L245
      return true;
    }
    return false;
  },
});
