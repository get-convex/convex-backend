import { HealthView } from "@common/features/health/components/HealthView";
import { useContext } from "react";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { Sheet } from "@ui/Sheet";
import { Link } from "@ui/Link";
import { Tooltip } from "@ui/Tooltip";
import { Spinner } from "@ui/Spinner";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { cn } from "@ui/cn";
import {
  CommandLineIcon,
  SignalIcon,
  WrenchIcon,
} from "@heroicons/react/24/outline";
import {
  CodeIcon,
  CubeIcon,
  ExternalLinkIcon,
  Pencil2Icon,
  RocketIcon,
} from "@radix-ui/react-icons";
import { deploymentTypeColorClasses } from "@common/lib/deploymentTypeColorClasses";

export default function HealthPage() {
  return (
    <HealthView
      header={<h3 className="sticky top-0 mx-6 pt-4 pb-2">Health</h3>}
      PagesWrapper={({ children }) => (
        <div className="flex min-h-0 grow">{children}</div>
      )}
      PageWrapper={({ children }) => (
        <div className="scrollbar max-w-full shrink-0 grow overflow-y-auto px-6 pb-4">
          {children}
        </div>
      )}
      summary={<OrchestratorSummary />}
    />
  );
}

// Summary panel mirroring cloud's DeploymentSummary layout: a left main
// panel (type chip + name, identity row, last-deployed row) and a right
// panel for the deployment URLs. Cloud's panel reads platform fields
// (region, class, expiry, backups) the orchestrator doesn't expose, so we
// surface only the subset that maps cleanly to a self-hosted deployment.
function OrchestratorSummary() {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  const lastPushEvent = useQuery(udfs.deploymentEvents.lastPushEvent, {});
  const serverVersion = useQuery(udfs.getVersion.default);

  if (!deployment) return null;

  const kind = (deployment.deploymentType ?? "prod") as
    | "prod"
    | "dev"
    | "preview"
    | "custom";

  // Cloud deployments expose `deploymentUrl`; local deployments expose
  // `port`. The orchestrator stores its deployments as kind="local" because
  // the actual backend runs on the user's host. We fall back through both
  // shapes so the same component renders for either.
  type AnyDeployment = {
    deploymentUrl?: string;
    port?: number;
  };
  const d = deployment as unknown as AnyDeployment;
  const baseUrl =
    d.deploymentUrl ?? (d.port ? `http://127.0.0.1:${d.port}` : null);
  const httpActionsUrl = baseUrl
    ? baseUrl.replace(/\/?$/, "").replace(/\.convex\.cloud$/, ".convex.site")
    : null;
  const port = d.port
    ? String(d.port)
    : baseUrl
      ? (() => {
          try {
            return new URL(baseUrl).port || null;
          } catch {
            return null;
          }
        })()
      : null;

  const isLoading = lastPushEvent === undefined || serverVersion === undefined;

  if (isLoading) {
    return (
      <Sheet className="flex w-fit flex-col bg-transparent" padding={false}>
        <div className="flex min-h-[7.5rem] min-w-[32rem] items-center justify-center rounded-lg bg-background-secondary p-2 py-3">
          <Spinner className="size-8" />
        </div>
      </Sheet>
    );
  }

  return (
    <Sheet className="flex w-fit flex-col bg-transparent" padding={false}>
      <div className="flex flex-col lg:flex-row">
        <div
          className={cn(
            "flex flex-col gap-4 bg-background-secondary p-2 py-3 lg:pr-4",
            "rounded-l-lg rounded-tr-lg rounded-bl-none lg:flex-1 lg:rounded-tr-none lg:rounded-bl-lg",
          )}
        >
          {/* Row 1: Type chip + deployment name */}
          <div className="flex flex-wrap items-center gap-2">
            <div
              className={cn(
                "inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium",
                deploymentTypeColorClasses(kind),
              )}
            >
              <KindIcon kind={kind} />
              <span>{kindLabel(kind)}</span>
            </div>
            <span className="font-mono text-sm text-content-secondary">
              {deployment.name}
            </span>
          </div>

          {/* Row 2: Port + Convex Version */}
          <div className="flex flex-wrap items-center gap-6">
            {port && (
              <div className="flex items-center gap-2">
                <Tooltip tip="Local port">
                  <CodeIcon
                    className="size-4 shrink-0 text-content-secondary"
                    aria-label="Port"
                  />
                </Tooltip>
                <div className="text-sm text-content-primary">Port {port}</div>
              </div>
            )}
            {serverVersion && (
              <div className="flex items-center gap-2">
                <Tooltip tip="Convex package version">
                  <CubeIcon
                    className="size-4 shrink-0 text-content-secondary"
                    aria-label="Convex package version"
                  />
                </Tooltip>
                <div className="text-sm text-content-primary">
                  Convex {serverVersion}
                </div>
              </div>
            )}
          </div>

          {/* Row 3: Last deployed */}
          <div className="flex flex-wrap items-center gap-2">
            <Tooltip tip="Last deployment">
              <RocketIcon
                className="size-4 shrink-0 text-content-secondary"
                aria-label="Last deployment"
              />
            </Tooltip>
            {!lastPushEvent ? (
              <span className="text-sm text-content-secondary">
                Never deployed
              </span>
            ) : (
              <div className="flex flex-wrap items-center gap-1 text-sm text-content-primary">
                <span>Last deployed</span>
                <TimestampDistance
                  date={new Date(lastPushEvent._creationTime)}
                  className="text-sm text-content-primary"
                />
              </div>
            )}
          </div>
        </div>

        {/* Deployment URLs panel */}
        {baseUrl && (
          <div className="flex flex-col justify-center gap-4 rounded-b-lg border-t bg-background-secondary/70 p-2 py-4 lg:rounded-r-lg lg:rounded-bl-none lg:border-t-0 lg:border-l lg:py-3 lg:pl-4">
            <div className="flex flex-col gap-1">
              <span className="text-xs font-medium text-content-secondary">
                Cloud URL
              </span>
              <Link
                href={baseUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="font-mono text-xs break-all"
                noUnderline
              >
                {baseUrl}
              </Link>
            </div>
            {httpActionsUrl && httpActionsUrl !== baseUrl && (
              <div className="flex flex-col gap-1">
                <span className="text-xs font-medium text-content-secondary">
                  HTTP Actions URL
                </span>
                <Link
                  href={httpActionsUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="font-mono text-xs break-all"
                  noUnderline
                >
                  {httpActionsUrl}
                </Link>
              </div>
            )}
            <div className="flex items-center gap-1 text-xs text-content-tertiary">
              <ExternalLinkIcon className="size-3.5" />
              <span>Self-hosted via convex-orchestrator</span>
            </div>
          </div>
        )}
      </div>
    </Sheet>
  );
}

function KindIcon({ kind }: { kind: "prod" | "dev" | "preview" | "custom" }) {
  if (kind === "prod") return <SignalIcon className="size-3.5" />;
  if (kind === "preview") return <Pencil2Icon className="size-3.5" />;
  if (kind === "custom") return <WrenchIcon className="size-3.5" />;
  return <CommandLineIcon className="size-3.5" />;
}

function kindLabel(kind: "prod" | "dev" | "preview" | "custom"): string {
  if (kind === "prod") return "Production";
  if (kind === "preview") return "Preview";
  if (kind === "custom") return "Custom";
  return "Development";
}
