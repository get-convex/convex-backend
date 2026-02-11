import { Card } from "elements/Card";
import { cn } from "@ui/cn";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { TeamMemberLink } from "elements/TeamMemberLink";
import { PlatformDeploymentResponse } from "@convex-dev/platform/managementApi";
import { ProjectDetails, MemberResponse } from "generatedApi";
import {
  getBackgroundColor,
  getDeploymentLabel,
} from "elements/DeploymentDisplay";
import {
  CommandLineIcon,
  SignalIcon,
  WrenchIcon,
} from "@heroicons/react/24/outline";
import { Pencil2Icon } from "@radix-ui/react-icons";

export function DeploymentCard({
  deployment,
  project,
  teamMembers,
  href,
  whoseName,
}: {
  deployment: PlatformDeploymentResponse;
  project: ProjectDetails;
  teamMembers?: MemberResponse[];
  href?: string;
  whoseName?: string | null;
}) {
  const creator = teamMembers?.find((tm) => tm.id === deployment.creator);
  const creatorName = creator?.name || creator?.email || "Unknown";

  return (
    <Card
      cardClassName="group min-w-fit animate-fadeInFromLoading"
      href={href}
      dropdownItems={undefined}
    >
      <div className="grid grid-cols-[minmax(200px,2fr)_minmax(150px,1.5fr)_minmax(150px,1.5fr)] gap-4">
        {/* First column: Deployment name and type */}
        <div className="flex flex-col gap-2">
          {/* Deployment name or port */}
          {deployment.kind === "cloud" && (
            <div className="truncate font-mono text-base font-semibold text-content-primary">
              {deployment.name}
            </div>
          )}
          {deployment.kind === "local" && (
            <div className="truncate text-base font-semibold text-content-primary">
              Port {deployment.port}
            </div>
          )}

          {/* Deployment type badge */}
          <div>
            <div
              className={cn(
                "inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium",
                getBackgroundColor(deployment.deploymentType),
              )}
            >
              <DeploymentIcon deployment={deployment} />
              <span className="truncate">
                {getDeploymentLabel({
                  deployment,
                  whoseName: whoseName ?? null,
                })}
              </span>
            </div>
          </div>
        </div>

        {/* Second column: Project name and slug */}
        <div className="flex min-w-0 flex-col items-start justify-center gap-0.5 text-left">
          <div className="w-full truncate text-sm text-content-primary">
            {project.name?.length ? project.name : "Untitled Project"}
          </div>
          <div className="w-full truncate text-xs text-content-secondary">
            {project.slug}
          </div>
        </div>

        {/* Third column: Created time and by */}
        <div className="flex items-center justify-end">
          <div className="flex flex-wrap items-center justify-end gap-1 text-right text-xs text-content-secondary">
            <TimestampDistance
              date={new Date(deployment.createTime)}
              className="truncate"
              prefix="Created"
            />
            {deployment.creator && (
              <>
                <span>by</span>
                <TeamMemberLink
                  memberId={deployment.creator}
                  name={creatorName}
                />
              </>
            )}
          </div>
        </div>
      </div>
    </Card>
  );
}

function DeploymentIcon({
  deployment,
}: {
  deployment: PlatformDeploymentResponse;
}) {
  if (deployment.deploymentType === "dev") {
    return <CommandLineIcon className="size-3.5" />;
  }
  if (deployment.deploymentType === "prod") {
    return <SignalIcon className="size-3.5" />;
  }
  if (deployment.deploymentType === "preview") {
    return <Pencil2Icon className="size-3.5" />;
  }
  if (deployment.deploymentType === "custom") {
    return <WrenchIcon className="size-3.5" />;
  }
  return null;
}
