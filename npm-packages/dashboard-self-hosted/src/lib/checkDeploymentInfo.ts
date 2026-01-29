import { joinUrlPath } from "@common/lib/helpers/joinUrlPath";

async function sleep(ms: number) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

const MAX_RETRIES = 3;
const MAX_RETRIES_DELAY_MS = 500;

export async function checkDeploymentInfo(
  adminKey: string,
  deploymentUrl: string,
): Promise<boolean | null> {
  let retries = 0;
  while (retries < MAX_RETRIES) {
    try {
      const resp = await fetch(
        joinUrlPath(deploymentUrl, "/api/check_admin_key"),
        {
          method: "GET",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Convex ${adminKey}`,
            "Convex-Client": "dashboard-0.0.0",
          },
        },
      );
      if (resp.ok) {
        return true;
      }
      if (resp.status === 404) {
        return null;
      }
    } catch {
      // Do nothing
    }
    await sleep(MAX_RETRIES_DELAY_MS);
    retries++;
  }
  return false;
}
