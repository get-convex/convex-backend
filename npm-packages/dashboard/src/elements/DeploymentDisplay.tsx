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
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { cn } from "lib/cn";
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
  const { localDeployments } = useLaunchDarkly();
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
        localDeployments,
        hasMultipleActiveLocalDeployments,
      })}
    </div>
  );
}

function getBackgroundColor(type: DeploymentType): string {
  switch (type) {
    case "prod":
      return "bg-purple-900 text-white";
    case "preview":
      return "bg-orange-400 text-black";
    case "dev":
      return "bg-green-600 text-white";
    default: {
      const _typecheck: never = type;
      return "";
    }
  }
}

function getDeploymentLabel({
  deployment,
  whoseName,
  localDeployments,
  hasMultipleActiveLocalDeployments,
}: {
  deployment: DeploymentResponse;
  whoseName: string | null; // null = mine
  localDeployments: boolean;
  hasMultipleActiveLocalDeployments: boolean;
}): string {
  switch (deployment.deploymentType) {
    case "prod":
      return "Production";
    case "preview":
      return `Preview: ${deployment.previewIdentifier || "Unknown"}`;
    case "dev": {
      if (localDeployments) {
        if (deployment.kind === "local") {
          return `${deployment.deviceName} ${hasMultipleActiveLocalDeployments ? `(Port ${deployment.port})` : ""}`;
        }
        return whoseName === null
          ? "Development (Cloud)"
          : `${whoseName}’s Dev (Cloud)`;
      }
      return whoseName === null ? "Development" : `${whoseName}’s Dev`;
    }
    default: {
      const _typecheck: never = deployment.deploymentType;
      return "";
    }
  }
}
