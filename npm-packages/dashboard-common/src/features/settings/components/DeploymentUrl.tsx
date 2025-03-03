import { ReactNode, useContext } from "react";
import Link from "next/link";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { CopyTextButton } from "@common/elements/CopyTextButton";

// dev/prod sometimes isn't initially loaded.
// Optimize for no flash on prod.
export function DeploymentType({
  deploymentType = "prod",
}: {
  deploymentType?: "prod" | "preview" | "dev";
}) {
  switch (deploymentType) {
    case "prod":
      return <span>production</span>;
    case "preview":
      return <span>preview</span>;
    case "dev":
      return <span>development</span>;
    default: {
      const _typeCheck: never = deploymentType;
      return null;
    }
  }
}

export function DeploymentUrl({ children }: { children: ReactNode }) {
  const convexCloudUrl = useQuery(udfs.convexCloudUrl.default, {});

  const { useCurrentDeployment } = useContext(DeploymentInfoContext);

  const deployment = useCurrentDeployment();

  // Use the cloud URL from the UDF, which includes custom domain overrides.
  // You could use `deploymentUrl` as a fallback, but to avoid confusion just
  // say "loading..." until the UDF result is available.
  const cloudUrl = convexCloudUrl ?? "loading...";

  return (
    <div className="text-content-primary">
      <h4 className="mb-4">Deployment URL</h4>
      <p className="mb-1">
        This <DeploymentType deploymentType={deployment?.deploymentType} />{" "}
        Convex deployment is hosted at the following URL.
      </p>
      <p className="mb-2">{children}</p>
      <CopyTextButton text={cloudUrl} className="text-sm font-normal" />
    </div>
  );
}

export function HttpActionsUrl() {
  const convexSiteUrl = useQuery(udfs.convexSiteUrl.default, {});

  const { useCurrentDeployment } = useContext(DeploymentInfoContext);

  const deployment = useCurrentDeployment();

  // Use the site URL from the UDF, which includes custom domain overrides.
  // You could derive a .convex.site url from `deploymentUrl` as a fallback,
  // but to avoid confusion just say "loading..." until the UDF result is
  // available.
  const httpActionsUrl = convexSiteUrl ?? "loading...";

  return (
    <div className="text-content-primary">
      <h4 className="mb-4">HTTP Actions URL</h4>
      <p className="mb-1">
        This <DeploymentType deploymentType={deployment?.deploymentType} />{" "}
        Convex deployment hosts{" "}
        <Link
          passHref
          href="https://docs.convex.dev/functions/http-actions"
          className="text-content-link"
          target="_blank"
        >
          HTTP Actions
        </Link>{" "}
        at the following URL.
      </p>
      <p className="mb-2">
        In Convex functions, this is available as{" "}
        <code>process.env.CONVEX_SITE_URL</code>.
      </p>
      <CopyTextButton text={httpActionsUrl} className="text-sm font-normal" />
    </div>
  );
}
