import { getGitHubHeaders } from "./github";

const AGENT_SKILLS_COMMITS_URL =
  "https://api.github.com/repos/get-convex/agent-skills/commits/main";

/**
 * Fetch the latest commit SHA from the get-convex/agent-skills repo.
 * Uses the `application/vnd.github.sha` media type so GitHub returns just the
 * bare 40-char SHA as plain text rather than a full JSON commit object.
 */
export async function getLatestAgentSkillsSha(): Promise<string> {
  const response = await fetch(AGENT_SKILLS_COMMITS_URL, {
    headers: {
      ...getGitHubHeaders(),
      Accept: "application/vnd.github.sha",
    },
  });

  if (!response.ok) {
    throw new Error(
      `GitHub API returned ${response.status}: ${await response.text()}`,
    );
  }

  const sha = (await response.text()).trim();
  if (!/^[0-9a-f]{40}$/i.test(sha)) {
    throw new Error(`Unexpected agent skills SHA format: ${sha}`);
  }

  return sha;
}
