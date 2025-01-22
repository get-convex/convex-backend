import Link from "next/link";
import { useRouter } from "next/router";

export function FunctionRunnerDisabledWhilePaused() {
  const { query } = useRouter();
  const teamSlug = query.team as string;
  const projectSlug = query.project as string;
  const deploymentName = query.deploymentName as string;
  return (
    <>
      The function runner is not available while the deployment is paused. To
      resume your deployment, go to{" "}
      <Link
        passHref
        href={`/t/${teamSlug}/${projectSlug}/${deploymentName}/settings/pause-deployment`}
        className="text-content-link underline hover:underline"
      >
        settings.
      </Link>
    </>
  );
}
