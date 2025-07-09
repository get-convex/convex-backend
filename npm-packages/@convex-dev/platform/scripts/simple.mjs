import path from "path";
import os from "os";
import { readFileSync } from "fs";

const CONVEX_PROVISION_HOST =
  process.env.CONVEX_PROVISION_HOST || "https://api.convex.dev";

function createHeaders(accessToken) {
  return {
    Authorization: `Bearer ${accessToken}`,
    "Content-Type": "application/json",
    "Convex-Client": "your-company-example-0.0.0",
  };
}

async function main(accessToken) {
  if (!accessToken) {
    console.log("No accessToken provided, using currently logged-in user...");
    const rootDir = path.join(os.homedir(), `.convex`);
    const globalConfig = path.join(rootDir, "config.json");
    accessToken = JSON.parse(
      readFileSync(globalConfig, { encoding: "utf-8" }),
    ).accessToken;
  }
  if (!accessToken) {
    throw new Error("Pass an accessToken or log in via `npx convex login`");
  }

  const headers = createHeaders(accessToken);

  console.log("Listing teams...");
  const teamsResponse = await fetch(
    `${CONVEX_PROVISION_HOST}/api/dashboard/teams`,
    {
      method: "GET",
      headers,
    },
  );

  if (!teamsResponse.ok) {
    console.log(`Error: ${teamsResponse.status} ${teamsResponse.statusText}`);
    return;
  }

  const teams = await teamsResponse.json();
  console.log(`${teams.length} teams`);

  const firstTeam = teams[0];
  if (!firstTeam) {
    console.log("No teams found");
    return;
  }

  console.log(firstTeam);
  console.log(`\nListing projects for team: ${firstTeam.name}`);

  const projectsResponse = await fetch(
    `${CONVEX_PROVISION_HOST}/api/dashboard/teams/${firstTeam.id}/projects`,
    {
      method: "GET",
      headers,
    },
  );

  if (!projectsResponse.ok) {
    console.log(
      `Error: ${projectsResponse.status} ${projectsResponse.statusText}`,
    );
    return;
  }

  const projects = await projectsResponse.json();
  console.log(`${projects.length} projects`);

  const createProjectResponse = await fetch(
    `${CONVEX_PROVISION_HOST}/api/dashboard/create_project`,
    {
      method: "POST",
      headers,
      body: JSON.stringify({
        projectName: `test-project-${Date.now()}`,
        team: firstTeam.slug,
        deploymentType: "dev",
      }),
    },
  );

  if (!createProjectResponse.ok) {
    console.error(
      `Error creating project: ${createProjectResponse.status} ${createProjectResponse.statusText}`,
    );
    return;
  }

  const projectData = await createProjectResponse.json();
  console.log("Project created!");
  console.log(`  - ${projectData.deploymentName}`);
  console.log(`  - URL: ${projectData.prodUrl}`);
  console.log(`  - slugs: ${projectData.teamSlug}/${projectData.projectSlug}`);
  console.log(`  - Admin Key: ${projectData.adminKey}`);
}

main().catch(console.error);
