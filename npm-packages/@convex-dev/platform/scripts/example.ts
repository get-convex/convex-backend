#!/usr/bin/env npx tsx

import path from "path";
import os from "os";
import { createConvexClient } from "../dist/index.js";
import { readFileSync } from "fs";

async function main(accessToken?: string) {
  if (!accessToken) {
    const rootDir = path.join(os.homedir(), `.convex`);
    const globalConfig = path.join(rootDir, "config.json");
    accessToken = JSON.parse(
      readFileSync(globalConfig, { encoding: "utf-8" }),
    ).accessToken;
  }
  if (!accessToken) {
    throw new Error("Pass an accessToken or log in via `npx convex login`");
  }

  const client = createConvexClient(accessToken);

  console.log("Listing teams...");
  const teamsResponse = await client.GET("/teams");
  console.log(
    teamsResponse.data
      ? `${teamsResponse.data.length} teams`
      : teamsResponse.error,
  );

  const firstTeam = teamsResponse.data?.[0];
  if (!firstTeam) {
    console.log("No teams found");
    return;
  }

  console.log(firstTeam);
  console.log(`\n🔍 Listing projects for team: ${firstTeam.name}`);

  const projectsResponse = await client.GET("/teams/{team_id}/projects", {
    params: {
      path: {
        team_id: "" + firstTeam.id,
      },
    },
  });

  console.log(
    projectsResponse.data
      ? `${projectsResponse.data.length} projects`
      : projectsResponse.error,
  );

  const createProjectResponse = await client.POST("/create_project", {
    body: {
      projectName: `test-project-${Date.now()}`,
      team: firstTeam.slug,
      deploymentType: "dev",
    },
  });

  if (createProjectResponse.error) {
    console.error("Error creating project:", createProjectResponse);
    return;
  }

  console.log("Project created successfully!");
  console.log(`  - ${createProjectResponse.data.deploymentName}`);
  console.log(`  - URL: ${createProjectResponse.data.prodUrl}`);
  console.log(
    `  - slugs: ${createProjectResponse.data.teamSlug}/${createProjectResponse.data.projectSlug}`,
  );
  console.log(`  - Admin Key: ${createProjectResponse.data.adminKey}`);
}

main().catch(console.error);
