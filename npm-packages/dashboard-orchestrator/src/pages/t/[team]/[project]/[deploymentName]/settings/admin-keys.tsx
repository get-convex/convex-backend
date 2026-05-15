import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { Sheet } from "@ui/Sheet";
import Link from "next/link";
import { useRouter } from "next/router";

export default function AdminKeys() {
  const router = useRouter();
  const { team, project } = router.query as { team?: string; project?: string };
  return (
    <DeploymentSettingsLayout page="admin-keys">
      <Sheet>
        <h3>Admin Keys</h3>
        <p className="mt-2 max-w-prose text-content-secondary">
          Admin keys for this deployment are managed at the project level.{" "}
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
