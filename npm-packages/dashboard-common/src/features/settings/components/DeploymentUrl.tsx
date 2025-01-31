import { ReactNode, useContext } from "react";
import Link from "next/link";
import { useQuery } from "convex/react";
import udfs from "udfs";
import { DeploymentInfoContext } from "lib/deploymentContext";
import { useDeploymentUrl } from "lib/deploymentApi";
import { CopyTextButton } from "elements/CopyTextButton";

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
  const deploymentUrl = useDeploymentUrl();

  const { useCurrentDeployment } = useContext(DeploymentInfoContext);

  const deployment = useCurrentDeployment();

  return (
    <div className="text-content-primary">
      <h4 className="mb-4">Deployment URL</h4>
      <p className="mb-1">
        This <DeploymentType deploymentType={deployment?.deploymentType} />{" "}
        Convex deployment is hosted at the following URL.
      </p>
      <p className="mb-2">{children}</p>
      <CopyTextButton text={deploymentUrl} className="text-sm font-normal" />
    </div>
  );
}

export function HttpActionsUrl() {
  const deploymentUrl = useDeploymentUrl();
  const convexSiteUrl = useQuery(udfs.convexSiteUrl.default, {});

  const { useCurrentDeployment } = useContext(DeploymentInfoContext);

  const deployment = useCurrentDeployment();

  // Use URL from UDF when available, which should be correct even when
  // running locally. Fall back to deriving from deployment URL if possible,
  // so the result is available immediately in most cases.
  const httpActionsUrl =
    convexSiteUrl !== undefined
      ? convexSiteUrl
      : deploymentUrl.endsWith(".cloud")
        ? `${deploymentUrl.slice(0, -".cloud".length)}.site`
        : "loading...";

  return (
    <div className="text-content-primary">
      <h4 className="mb-4">HTTP Actions URL</h4>
      <p className="mb-1">
        This <DeploymentType deploymentType={deployment?.deploymentType} />{" "}
        Convex deployment hosts{" "}
        <Link
          passHref
          href="https://docs.convex.dev/functions/http-actions"
          className="text-content-link dark:underline"
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
