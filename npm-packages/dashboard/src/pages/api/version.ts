import type { NextApiRequest, NextApiResponse } from "next";

const teamId = "team_o9VFxgDgFFRw2zuhlvo51gCY";
const projectId = "prj_0WgHdhLOoHKNyydKfSKAWXeAyGzR";
const productionBranchUrl = "dashboard.convex.dev";

export default async function handler(
  request: NextApiRequest,
  response: NextApiResponse<{}>,
) {
  if (!process.env.VERCEL_TOKEN) {
    console.error("VERCEL_TOKEN not set");
    response.status(500).json({ error: "Failed to fetch version information" });
    return;
  }

  if (process.env.VERCEL_ENV !== "preview") {
    // If we're not in a preview deployment, fetch the production deployment from Vercel's undocument production-deployment API.
    // This response includes a boolean indicating if the production deployment is stale (i.e. rolled back)
    // We accept the risk that this API might change because it is not documented, meaning the version
    // notification feature might silently fail. We would get errors in sentry in this case, but user's
    // wouldn't notice.
    const prodResponse = await fetch(
      `https://vercel.com/api/v1/projects/${projectId}/production-deployment?teamId=${teamId}`,
      {
        headers: {
          Authorization: `Bearer ${process.env.VERCEL_TOKEN}`,
        },
        method: "get",
      },
    );
    if (!prodResponse.ok) {
      response
        .status(500)
        .json({ error: "Failed to fetch production version information" });
      return;
    }

    const prodData = await prodResponse.json();

    // Since we retrieved data from an undocumented API
    // let's defensively check that the data we need is present
    // and return an opaque error if it isn't.
    if (!prodData || typeof prodData.deploymentIsStale !== "boolean") {
      response
        .status(500)
        .json({ error: "Failed to fetch production deployment" });
      return;
    }

    // If the production deployment is rolled back,
    // we should not show a version notification.
    if (prodData.deploymentIsStale) {
      response.status(200).json({ sha: null });
      return;
    }
  }

  // Even though we retrieved the production data, we might be on a preview branch deployment.
  // So, fetch the data specific to the latest branch deployment.
  const branchResponse = await fetch(
    `https://api.vercel.com/v13/deployments/${process.env.VERCEL_BRANCH_URL || productionBranchUrl}?teamId=${teamId}`,
    {
      headers: {
        Authorization: `Bearer ${process.env.VERCEL_TOKEN}`,
      },
      method: "get",
    },
  );
  if (!branchResponse.ok) {
    response
      .status(500)
      .json({ error: "Failed to fetch branch version information" });
    return;
  }

  const branchData = await branchResponse.json();

  response.status(200).json({
    sha: branchData.gitSource.sha,
  });
}
