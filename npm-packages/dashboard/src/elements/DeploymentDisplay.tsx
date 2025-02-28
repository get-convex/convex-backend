import { CommandLineIcon, SignalIcon } from "@heroicons/react/20/solid";
import {
  GlobeIcon,
  MixerHorizontalIcon,
  Pencil2Icon,
} from "@radix-ui/react-icons";
import { useCurrentDeployment, useDeployments } from "api/deployments";
import { useCurrentTeam, useTeamMembers } from "api/teams";
import { useProjectById } from "api/projects";
import { useProfile } from "api/profile";
import { useRememberLastViewedDeployment } from "hooks/useLastViewed";
import { cn } from "dashboard-common/lib/cn";
import { useRouter } from "next/router";
import {
  DeploymentResponse,
  ProjectDetails,
  DeploymentType,
} from "generatedApi";

export function DeploymentDisplay({ project }: { project: ProjectDetails }) {
  const router = useRouter();

  const deployment = useCurrentDeployment();
  const member = useProfile();

  useRememberLastViewedDeployment(deployment?.name);

  const teamMembers = useTeamMembers(project.teamId);

  const isProdSelected = deployment?.deploymentType === "prod";
  const isPreview = deployment?.deploymentType === "preview";
  const isDoneLoading =
    isProdSelected || // prod deploys are straightforward
    isPreview || // preview deployments don't require additional info
    (deployment !== undefined &&
      member !== undefined &&
      teamMembers !== undefined); // info required to render dev deploy

  const whose = teamMembers?.find((tm) => tm.id === deployment?.creator);
  const profile = useProfile();
  const whoseName =
    whose?.email === profile?.email
      ? profile?.name || profile?.email
      : whose?.name || whose?.email || "Teammate";

  const isProjectSettings = router.route.endsWith("/[project]/settings");

  return isProjectSettings ? (
    <div className="flex items-center gap-2">
      <MixerHorizontalIcon />
      <span className="hidden sm:block">Project settings</span>
    </div>
  ) : !isDoneLoading ? null : whoseName ? (
    <DeploymentLabel deployment={deployment} whoseName={whoseName} />
  ) : null;
}

export function DeploymentLabel({
  whoseName,
  deployment,
  inline = false,
}: {
  deployment: DeploymentResponse;
  whoseName: string | null;
  inline?: boolean;
}) {
  const team = useCurrentTeam();
  const project = useProjectById(team?.id, deployment.projectId);
  const { deployments } = useDeployments(project?.id);
  const hasMultipleActiveLocalDeployments =
    deployments !== undefined &&
    deployments.filter(
      (d) => d.deploymentType === "dev" && d.kind === "local" && d.isActive,
    ).length > 1;
  return (
    <div
      className={cn(
        "flex items-center gap-2 rounded-md",
        !inline && getBackgroundColor(deployment.deploymentType),
        !inline && "p-1",
      )}
    >
      {deployment.deploymentType === "dev" ? (
        deployment.kind === "local" ? (
          <CommandLineIcon className="size-4" />
        ) : (
          <GlobeIcon className="size-4" />
        )
      ) : deployment.deploymentType === "prod" ? (
        <SignalIcon className="size-4" />
      ) : deployment.deploymentType === "preview" ? (
        <Pencil2Icon className="size-4" />
      ) : null}
      {getDeploymentLabel({
        deployment,
        whoseName,
        hasMultipleActiveLocalDeployments,
      })}
    </div>
  );
}

function getBackgroundColor(type: DeploymentType): string {
  switch (type) {
    case "prod":
      return "border border-purple-600 dark:border-purple-900 bg-purple-100/50 text-purple-600 dark:bg-purple-900/50 dark:text-purple-400";
    case "preview":
      return "border border-orange-400 bg-orange-100/50 text-orange-600 dark:bg-orange-900/50 dark:text-orange-400";
    case "dev":
      return "border border-green-600 bg-green-100/50 text-green-600 dark:bg-green-900/50 dark:text-green-400";
    default: {
      const _typecheck: never = type;
      return "";
    }
  }
}

function getDeploymentLabel({
  deployment,
  whoseName,
  hasMultipleActiveLocalDeployments,
}: {
  deployment: DeploymentResponse;
  whoseName: string | null; // null = mine
  hasMultipleActiveLocalDeployments: boolean;
}): string {
  switch (deployment.deploymentType) {
    case "prod":
      return "Production";
    case "preview":
      return `Preview: ${deployment.previewIdentifier || "Unknown"}`;
    case "dev": {
      if (deployment.kind === "local") {
        return `${deployment.deviceName} ${hasMultipleActiveLocalDeployments ? `(Port ${deployment.port})` : ""}`;
      }
      return whoseName === null
        ? "Development (Cloud)"
        : `${whoseName}’s Dev (Cloud)`;
    }
    default: {
      const _typecheck: never = deployment.deploymentType;
      return "";
    }
  }
}
