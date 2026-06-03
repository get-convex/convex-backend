import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { CopyButton } from "@common/elements/CopyButton";
import { Link } from "@ui/Link";
import { Sheet } from "@ui/Sheet";
import { Spinner } from "@ui/Spinner";
import udfs from "@common/udfs";
import { useQuery } from "convex/react";

export default function CustomDomainsPage() {
  const cloudUrl = useQuery(udfs.convexCloudUrl.default, {});
  const siteUrl = useQuery(udfs.convexSiteUrl.default, {});

  return (
    <DeploymentSettingsLayout page="custom-domains">
      <Sheet>
        <h3>Custom Domains</h3>
        <p className="mt-2 max-w-prose text-content-primary">
          On self-hosted deployments, custom domains are configured at the
          backend process level via the{" "}
          <code className="rounded-sm bg-background-tertiary px-1 text-xs">
            CONVEX_CLOUD_ORIGIN
          </code>{" "}
          and{" "}
          <code className="rounded-sm bg-background-tertiary px-1 text-xs">
            CONVEX_SITE_ORIGIN
          </code>{" "}
          environment variables. To change either, update the environment in
          your container or process supervisor and restart the backend. See the{" "}
          <Link
            href="https://github.com/get-convex/convex-backend/blob/main/self-hosted/advanced/hosting_on_own_infra.md"
            target="_blank"
          >
            self-hosting guide
          </Link>{" "}
          for details.
        </p>

        <div className="mt-4 flex flex-col gap-3">
          <UrlRow
            envVar="CONVEX_CLOUD_ORIGIN"
            label="Deployment URL"
            value={cloudUrl}
          />
          <UrlRow
            envVar="CONVEX_SITE_ORIGIN"
            label="HTTP Actions URL"
            value={siteUrl}
          />
        </div>
      </Sheet>
    </DeploymentSettingsLayout>
  );
}

function UrlRow({
  envVar,
  label,
  value,
}: {
  envVar: string;
  label: string;
  value: string | undefined;
}) {
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-baseline gap-2">
        <span className="text-sm font-medium text-content-primary">
          {label}
        </span>
        <code className="text-xs text-content-secondary">{envVar}</code>
      </div>
      {value === undefined ? (
        <Spinner className="size-4" />
      ) : (
        <div className="flex items-center gap-2">
          <code className="min-w-0 flex-1 truncate rounded-sm bg-background-tertiary px-2 py-1 font-mono text-sm">
            {value}
          </code>
          <CopyButton text={value} />
        </div>
      )}
    </div>
  );
}
