import type { GetServerSideProps, GetServerSidePropsContext } from "next";
import { getAccessToken, withPageAuthRequired } from "server/workos";
import groupBy from "lodash/groupBy";
import { DeploymentResponse, TeamResponse, ProjectDetails } from "generatedApi";
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
  const shouldRedirectToProjectPageFromDeployment =
    resolvedUrl.startsWith("/dp/");

  // If this is not the first time we're loading props, we can return early
  // and not re-fetch additional data.
  if (
    !isFirstServerCall &&
    !shouldRedirectToDeploymentPage &&
    !shouldRedirectToProjectPage &&
    !shouldRedirectToProjectPageFromDeployment
  ) {
    return { props: {} };
  }

  let token: { accessToken: string } | null;
  try {
    token = await getAccessToken(req);
  } catch (error) {
    console.error("Couldn't fetch auth token", error);
    // If we can't get the token, we should try to login again
    res.writeHead(307, {
      Location: `/api/auth/login?returnTo=${req.url}`,
    });
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
    const headers: Record<string, string> = {
      authorization: `Bearer ${token.accessToken}`,
      "Google-Analytics-Client-Id": googleAnalyticsId,
    };
    const resp = await retryingFetch(
      `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard/member_data`,
      {
        headers,
      },
    );
    if (!resp.ok) {
      try {
        const error: { message: string; code: string } = await resp.json();
        if (
          error.code === "InvalidIdentity" &&
          !resolvedUrl.startsWith("/link_identity")
        ) {
          const hint = error.message.split("(hint:")[1]?.split(")")[0];
          res.writeHead(307, {
            Location: `/link_identity?returnTo=${req.url}&hint=${hint}`,
          });
          res.end();
          return { props: {} };
        }
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
      teams: TeamResponse[];
      projects: ProjectDetails[];
      deployments: DeploymentResponse[];
      optInsToAccept?: {
        optIn: string;
        message: string;
      }[];
    } = await resp.json();
    const { team, project, deploymentName } = query;
    if (team && !teams.find((t: TeamResponse) => t.slug === team.toString())) {
      // You're looking for a page that doesn't exist!
      return pageNotFound(res);
    }

    if (shouldRedirectToProjectPage && project !== undefined) {
      return await redirectToProjectPage(
        resolvedUrl,
        res,
        project as string,
        token.accessToken,
      );
    }

    if (shouldRedirectToDeploymentPage && deploymentName !== undefined) {
      return await redirectToDeploymentPage(
        resolvedUrl,
        res,
        deploymentName as string,
        token.accessToken,
      );
    }

    if (
      shouldRedirectToProjectPageFromDeployment &&
      deploymentName !== undefined
    ) {
      return await redirectToProjectPageFromDeploymentName(
        resolvedUrl,
        res,
        deploymentName as string,
        token.accessToken,
      );
    }

    const projectsByTeam = groupBy(projects, (p: ProjectDetails) => p.teamId);

    const initialProjectsByTeam = Object.fromEntries(
      teams.map(({ id: teamId }) => [
        `/teams/${teamId}/projects`,
        projectsByTeam[teamId] ?? [],
      ]),
    );

    const initialIndividualProjects = Object.fromEntries(
      projects.map(({ id: projectId }) => [
        `/projects/${projectId}`,
        projects.find((p: ProjectDetails) => p.id === projectId),
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
      ...initialProjectsByTeam,
      ...initialIndividualProjects,
      ...initialDeployments,
    };

    if (optInsToAccept !== undefined) {
      initialData["/optins"] = { optInsToAccept };
    }

    return {
      props: {
        accessToken: token.accessToken,
        initialData,
      },
      redirect:
        optInsToAccept &&
        optInsToAccept.length > 0 &&
        !resolvedUrl.startsWith("/accept")
          ? {
              destination: `/accept${resolvedUrl ? `?from=${encodeURIComponent(resolvedUrl)}` : ""}`,
              permanent: false,
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

async function redirectToProjectPage(
  resolvedUrl: string,
  res: GetServerSidePropsContext["res"],
  projectSlug: string,
  accessToken: string,
) {
  try {
    // Fetch all teams to find which team owns this project
    const teamsResp = await retryingFetch(
      `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard/teams`,
      {
        headers: {
          authorization: `Bearer ${accessToken}`,
        },
      },
    );

    if (!teamsResp.ok) {
      return pageNotFound(res);
    }

    const teams: TeamResponse[] = await teamsResp.json();

    // Try each team to find the project
    let project: ProjectDetails | undefined;
    let owningTeam: TeamResponse | undefined;

    for (const team of teams) {
      try {
        const projectResp = await retryingFetch(
          `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard/teams/${team.id}/projects/${projectSlug}`,
          {
            headers: {
              authorization: `Bearer ${accessToken}`,
            },
          },
        );

        if (projectResp.ok) {
          project = await projectResp.json();
          owningTeam = team;
          break;
        }
      } catch (error) {
        // Continue to next team
        continue;
      }
    }

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
  } catch (error) {
    return pageNotFound(res);
  }
}

async function redirectToDeploymentPage(
  resolvedUrl: string,
  res: GetServerSidePropsContext["res"],
  deploymentName: string,
  accessToken: string,
) {
  try {
    // Fetch team and project info for the deployment in a single call
    const teamAndProjectResp = await retryingFetch(
      `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/deployment/${deploymentName}/team_and_project`,
      {
        headers: {
          authorization: `Bearer ${accessToken}`,
        },
      },
    );

    if (!teamAndProjectResp.ok) {
      return pageNotFound(res);
    }

    const { team, project }: { team: string; project: string } =
      await teamAndProjectResp.json();

    const remainingPath = resolvedUrl.slice(`/d/${deploymentName}`.length);
    return {
      redirect: {
        permanent: false,
        destination: `/t/${team}/${project}/${deploymentName}${remainingPath}`,
      },
    };
  } catch (error) {
    console.error("Error redirecting to deployment page", error);
    return pageNotFound(res);
  }
}

async function redirectToProjectPageFromDeploymentName(
  resolvedUrl: string,
  res: GetServerSidePropsContext["res"],
  deploymentName: string,
  accessToken: string,
) {
  try {
    // Fetch team and project info for the deployment in a single call
    const teamAndProjectResp = await retryingFetch(
      `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/deployment/${deploymentName}/team_and_project`,
      {
        headers: {
          authorization: `Bearer ${accessToken}`,
        },
      },
    );

    if (!teamAndProjectResp.ok) {
      return pageNotFound(res);
    }

    const { team, project }: { team: string; project: string } =
      await teamAndProjectResp.json();

    const remainingPath = resolvedUrl.slice(`/dp/${deploymentName}`.length);
    return {
      redirect: {
        permanent: false,
        destination: `/t/${team}/${project}${remainingPath}`,
      },
    };
  } catch (error) {
    console.error("Error redirecting to project page from deployment", error);
    return pageNotFound(res);
  }
}

function pageNotFound(res: GetServerSidePropsContext["res"]) {
  res.writeHead(307, { Location: "/404" });
  res.end();
  return { props: {} };
}

export const retryingFetch = fetchRetryFactory(fetch, {
  retries: 2,
  retryDelay: (attempt: number, _error: any, _response: any) => {
    // immediate, 1s delay, 2s delay, 4s delay, etc.
    const delay = attempt === 0 ? 1 : 2 ** (attempt - 1) * 1000;
    const randomSum = delay * 0.2 * Math.random();
    return delay + randomSum;
  },
  retryOn(_attempt: number, error: Error | null, response: Response | null) {
    // Don't retry on client errors (4xx)
    if (response && response.status >= 400 && response.status < 500) {
      return false;
    }
    // Retry on network errors.
    if (error) {
      // TODO filter out all SSL errors
      // https://github.com/nodejs/node/blob/8a41d9b636be86350cd32847c3f89d327c4f6ff7/src/crypto/crypto_common.cc#L218-L245
      return true;
    }
    // Retry on 5xx server errors
    if (response && response.status >= 500) {
      return true;
    }
    return false;
  },
});
