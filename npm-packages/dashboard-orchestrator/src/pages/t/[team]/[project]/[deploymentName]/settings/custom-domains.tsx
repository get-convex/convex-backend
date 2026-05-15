import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { Sheet } from "@ui/Sheet";
import Link from "next/link";
import { useRouter } from "next/router";

export default function CustomDomains() {
  const router = useRouter();
  const { team, project } = router.query as { team?: string; project?: string };
  return (
    <DeploymentSettingsLayout page="custom-domains">
      <Sheet>
        <h3>Custom Domains</h3>
        <p className="mt-2 max-w-prose text-content-secondary">
          Custom domains are managed by the orchestrator at the project level.{" "}
          {team && project && (
            <Link href={`/t/${team}/${project}/settings`} className="underline">
              Open project settings
            </Link>
          )}
        </p>
      </Sheet>
    </DeploymentSettingsLayout>
  );
}
