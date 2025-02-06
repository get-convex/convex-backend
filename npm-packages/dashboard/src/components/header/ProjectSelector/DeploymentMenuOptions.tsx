import { useDefaultDevDeployment, useDeployments } from "api/deployments";
import { useTeamMembers, useTeamEntitlements } from "api/teams";
import { useProfile } from "api/profile";
import Link from "next/link";
import { useRouter } from "next/router";
import React, { useRef } from "react";
import { CommandLineIcon, SignalIcon } from "@heroicons/react/20/solid";
import { Tooltip } from "dashboard-common/elements/Tooltip";
import { SelectorItem } from "elements/SelectorItem";
import {
  ChevronDownIcon,
  ChevronUpIcon,
  ExternalLinkIcon,
  GlobeIcon,
  Pencil2Icon,
} from "@radix-ui/react-icons";
import { DeploymentResponse, ProjectDetails, Team } from "generatedApi";
import { Disclosure } from "@headlessui/react";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { logEvent } from "convex-analytics";
import { PROVISION_PROD_PAGE_NAME } from "dashboard-common/lib/deploymentContext";
import { useIsOverflowing } from "dashboard-common/lib/useIsOverflowing";

export function DeploymentMenuOptions({
  team,
  project,
  close,
}: {
  team: Team;
  project: ProjectDetails;
  close: () => void;
}) {
  const member = useProfile();
  const router = useRouter();
  const arePreviewDeploymentsAvailable =
    useTeamEntitlements(team.id)?.projectMaxPreviewDeployments !== 0;

  const { deployments: deploymentData } = useDeployments(project.id);
  const deployments = deploymentData || [];

  const previews = deployments
    .filter((d) => d.deploymentType === "preview")
    .sort((a, b) => b.createTime - a.createTime);

  const members = useTeamMembers(team.id);
  const teamMemberDeployments = deployments
    .filter(
      (d) =>
        d.kind === "cloud" &&
        d.deploymentType === "dev" &&
        d.creator !== member?.id,
    )
    .map((d) => {
      const whose = members?.find((tm) => tm.id === d.creator);
      return {
        name: d.name,
        creator: whose?.name || whose?.email || "Teammate",
      };
    })
    .sort((a, b) => a.creator.localeCompare(b.creator));

  const projectSlug = project.slug;

  const selectedTeamSlug = team.slug;

  const prod = deployments?.find((d) => d.deploymentType === "prod");

  const projectsURI = `/t/${selectedTeamSlug}/${projectSlug}`;
  // 0-4 are /t/[team]/[project]/[deploymentName].
  // 5- is the currentView
  const currentView = router.asPath.split("/").slice(5).join("/");
  return (
    <div className="mx-0.5 mb-2">
      <SelectorItem
        className="flex items-center gap-2"
        selected={
          !!router.query.deploymentName &&
          router.query.deploymentName === prod?.name
        }
        disabled={project.isDemo}
        href={`${projectsURI}/${
          prod === undefined
            ? PROVISION_PROD_PAGE_NAME
            : `${prod?.name}/${currentView}`
        }`}
        close={close}
        eventName="switch to production deployment"
      >
        <SignalIcon className="h-4 w-4" />
        Production
      </SelectorItem>
      <AllPersonalDeployments team={team} project={project} close={close} />
      {previews.length === 0 && (
        <Tooltip
          className="w-full"
          side="right"
          tip={
            <NoPreviewTooltip
              isDemo={project.isDemo}
              arePreviewDeploymentsAvailable={arePreviewDeploymentsAvailable}
              teamSlug={selectedTeamSlug!}
            />
          }
        >
          <SelectorItem
            className="flex items-center gap-2"
            disabled={project.isDemo}
            href="https://docs.convex.dev/production/hosting/preview-deployments"
            target="_blank"
            close={close}
            eventName="open preview deployment docs"
          >
            <Pencil2Icon className="h-4 w-4" />
            Preview
            <ExternalLinkIcon className="ml-auto h-4 w-4" />
          </SelectorItem>
        </Tooltip>
      )}
      {previews.length > 0 && (
        <Disclosure>
          {({ open }) => (
            <>
              <Disclosure.Button
                className="w-full pr-2"
                onClick={() => logEvent("toggle preview deployments")}
              >
                <div className="flex w-full items-center justify-between gap-2">
                  <div className="p-2 text-xs text-content-secondary">
                    Preview Deployments ({previews.length})
                  </div>
                  {open ? <ChevronUpIcon /> : <ChevronDownIcon />}
                </div>
              </Disclosure.Button>
              <Disclosure.Panel className="w-full">
                {previews.map((previewDeployment) => (
                  <SelectorItem
                    close={close}
                    href={`${projectsURI}/${previewDeployment.name}/${currentView}`}
                    selected={
                      router.query.deploymentName === previewDeployment.name
                    }
                    eventName="switch to preview deployment"
                  >
                    <DeploymentOption
                      identifier={
                        previewDeployment.previewIdentifier ??
                        previewDeployment.name
                      }
                    />
                  </SelectorItem>
                ))}
              </Disclosure.Panel>
            </>
          )}
        </Disclosure>
      )}
      {teamMemberDeployments.length > 0 && (
        <Disclosure>
          {({ open }) => (
            <>
              <Disclosure.Button
                className="w-full pr-2"
                onClick={() => logEvent("toggle other deployments")}
              >
                <div className="flex w-full items-center justify-between gap-2">
                  <div className="p-2 text-xs text-content-secondary">
                    Other Deployments ({teamMemberDeployments.length})
                  </div>
                  {open ? <ChevronUpIcon /> : <ChevronDownIcon />}
                </div>
              </Disclosure.Button>
              <Disclosure.Panel className="w-full">
                {teamMemberDeployments.map((d) => (
                  <SelectorItem
                    close={close}
                    href={`${projectsURI}/${d.name}/${currentView}`}
                    selected={router.query.deploymentName === d.name}
                    eventName="switch to other deployment"
                  >
                    <DeploymentOption identifier={`${d.creator}'s Dev`} />
                  </SelectorItem>
                ))}
              </Disclosure.Panel>
            </>
          )}
        </Disclosure>
      )}
    </div>
  );
}

function AllPersonalDeployments({
  team,
  project,
  close,
}: {
  project: ProjectDetails;
  team: Team;
  close: () => void;
}) {
  const { localDeployments } = useLaunchDarkly();
  const member = useProfile();
  const dev = useDefaultDevDeployment(project.id, localDeployments);
  const router = useRouter();
  const projectSlug = project.slug;

  const selectedTeamSlug = team.slug;

  const projectsURI = `/t/${selectedTeamSlug}/${projectSlug}`;
  // 0-4 are /t/[team]/[project]/[deploymentName].
  // 5- is the currentView
  const currentView = router.asPath.split("/").slice(5).join("/");
  const deployments = useDeployments(project.id).deployments || [];
  const allDevDeployments = sortDevDeployments(
    deployments.filter(
      (d) => d.deploymentType === "dev" && d.creator === member?.id,
    ),
  );
  const hasMultipleActiveLocalDeployments =
    allDevDeployments.filter((d) => d.kind === "local" && d.isActive).length >
    1;

  if (!localDeployments) {
    return (
      <Tooltip
        className="w-full"
        side="right"
        tip={dev ? undefined : <NoDevTooltip />}
      >
        <SelectorItem
          className="flex items-center gap-2"
          selected={
            !!router.query.deploymentName &&
            router.query.deploymentName === dev?.name
          }
          disabled={!dev}
          href={`${projectsURI}/${dev?.name}/${currentView}`}
          close={close}
          eventName="switch to cloud dev deployment"
        >
          <CommandLineIcon className="h-4 w-4" />
          Development
        </SelectorItem>
      </Tooltip>
    );
  }
  if (allDevDeployments.length === 0) {
    <Tooltip className="w-full" side="right" tip={<NoDevTooltip />}>
      <SelectorItem
        className="flex items-center gap-2"
        selected={false}
        disabled
        href={`${projectsURI}/${dev?.name}/${currentView}`}
        close={close}
      >
        <CommandLineIcon className="h-4 w-4" />
        Development
      </SelectorItem>
    </Tooltip>;
  }
  // TODO(sarah) - consider adding a tooltip around inactive local deployments
  return (
    <>
      {allDevDeployments.map((d) => (
        <SelectorItem
          key={d.name}
          className="flex items-center gap-2"
          close={close}
          disabled={d.kind === "local" && d.isActive === false}
          href={`${projectsURI}/${d.name}/${currentView}`}
          selected={router.query.deploymentName === d.name}
          eventName={
            d.kind === "local"
              ? "switch to local dev deployment"
              : "switch to cloud dev deployment"
          }
        >
          {d.kind === "local" ? (
            <CommandLineIcon className="h-4 w-4" />
          ) : (
            <GlobeIcon className="h-4 w-4" />
          )}
          <DeploymentOption
            identifier={`${d.kind === "local" ? `${d.deviceName} ${hasMultipleActiveLocalDeployments ? `(Port ${d.port})` : ""}` : "Development (Cloud)"}`}
          />
        </SelectorItem>
      ))}
    </>
  );
}

function sortDevDeployments(deployments: DeploymentResponse[]) {
  return deployments.sort((a, b) => {
    // Sort inactive local deployments to the end
    if (a.kind === "local" && !a.isActive) {
      return 1;
    }
    if (b.kind === "local" && !b.isActive) {
      return -1;
    }

    // Sort local deployments before cloud deployments
    if (a.kind === "local" && b.kind === "cloud") {
      return -1;
    }
    if (a.kind === "cloud" && b.kind === "local") {
      return 1;
    }

    // Sort by last update time for local deployments
    if (a.kind === "local" && b.kind === "local") {
      return a.lastUpdateTime - b.lastUpdateTime;
    }
    return a.createTime - b.createTime;
  });
}

function DeploymentOption({ identifier }: { identifier: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const isOverflowing = useIsOverflowing(ref);

  return (
    <Tooltip
      tip={
        isOverflowing ? (
          <div className="break-all">{identifier}</div>
        ) : undefined
      }
      side="right"
      wrapsButton
    >
      <p className="max-w-[14rem] truncate" ref={ref}>
        {identifier}
      </p>
    </Tooltip>
  );
}

function NoDevTooltip() {
  return (
    <div>
      You do not have a personal development environment in this project yet.{" "}
      <Link
        passHref
        href="https://docs.convex.dev/cli#run-the-convex-dev-server"
        className="underline"
        target="_blank"
      >
        Learn more
      </Link>
    </div>
  );
}

function NoPreviewTooltip({
  isDemo,
  arePreviewDeploymentsAvailable,
  teamSlug,
}: {
  isDemo: boolean;
  arePreviewDeploymentsAvailable: boolean;
  teamSlug: string;
}) {
  if (isDemo) {
    return <div>Create a new project to use preview deployments.</div>;
  }
  if (arePreviewDeploymentsAvailable) {
    return (
      <div>
        You do not have any preview deployments for this project yet.{" "}
        <Link
          passHref
          href="https://docs.convex.dev/production/hosting/preview-deployments"
          className="underline"
          target="_blank"
        >
          Learn more
        </Link>
      </div>
    );
  }
  return (
    <div>
      <Link
        passHref
        href="https://docs.convex.dev/production/hosting/preview-deployments"
        className="underline"
        target="_blank"
      >
        Preview deployments
      </Link>
      {" are only available in paid plans. "}
      <Link href={`/${teamSlug}/settings/billing`} className="underline">
        Upgrade to get access.
      </Link>
    </div>
  );
}
