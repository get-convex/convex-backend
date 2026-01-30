import { EyeOpenIcon, TrashIcon, GearIcon } from "@radix-ui/react-icons";
import { Card, CardProps } from "elements/Card";
import { Tooltip } from "@ui/Tooltip";
import { Loading } from "@ui/Loading";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { LostAccessModal } from "components/projects/modals/LostAccessModal";
import { useDeploymentUris } from "hooks/useDeploymentUris";
import classNames from "classnames";
import Link from "next/link";
import { useRouter } from "next/router";
import { ReactNode, useState } from "react";
import { ProjectDetails } from "generatedApi";
import { useHasProjectAdminPermissions } from "api/roles";
import { useCurrentTeam } from "api/teams";
import { DeleteProjectModal } from "./modals/DeleteProjectModal";

export function ProjectCard({ project }: { project: ProjectDetails }) {
  const router = useRouter();
  const { id, slug, name } = project;
  const team = useCurrentTeam();

  const [deleteModal, setDeleteModal] = useState(false);
  const [lostAccessModal, setLostAccessModal] = useState(false);

  const {
    prodHref,
    devHref,
    isProdDefault,
    defaultHref,
    hasDefaultProdDeployment,
    hasDefaultDevDeployment,
    isLoading: isLoadingDeployments,
  } = useDeploymentUris(id, slug);

  const hasAdminPermissions = useHasProjectAdminPermissions(project.id);

  function openSettings() {
    void router.push(`/t/${team?.slug}/${project.slug}/settings`);
  }

  const dropdownItems: CardProps["dropdownItems"] = [
    {
      Icon: GearIcon,
      text: "Settings",
      action: openSettings,
    },

    {
      Icon: EyeOpenIcon,
      text: "Lost Access",
      action: () => setLostAccessModal(true),
    },
    {
      Icon: TrashIcon,
      destructive: true,
      text: "Delete project",
      action: () => setDeleteModal(true),
      disabled: !hasAdminPermissions,
      tip: !hasAdminPermissions
        ? "You do not have permission to delete this project."
        : undefined,
    },
  ];

  return (
    <Card
      cardClassName="group animate-fadeInFromLoading"
      href={deleteModal || lostAccessModal ? undefined : defaultHref}
      dropdownItems={dropdownItems}
      overlayed={
        <div className="flex gap-1">
          {!isLoadingDeployments ? (
            <div className="flex flex-col items-end">
              <DeploymentLinks
                isProdDefault={isProdDefault}
                devHref={devHref}
                prodHref={prodHref}
                hasDefaultDevDeployment={hasDefaultDevDeployment}
                hasDefaultProdDeployment={hasDefaultProdDeployment}
              />
              <TimestampDistance
                date={new Date(project.createTime)}
                className="truncate"
                prefix="Created"
              />
            </div>
          ) : (
            <Loading className="min-h-6 w-36" />
          )}
        </div>
      }
    >
      <div>
        <div
          className={classNames(
            "truncate",
            !name ? "text-content-secondary" : "",
          )}
        >
          <span className="flex items-center gap-2 text-content-primary">
            <span className="shrink truncate">
              {name?.length ? name : "Untitled Project"}
            </span>
          </span>
        </div>
        <div className="mb-1 h-4 truncate text-xs text-content-secondary">
          {slug}
        </div>
        {team && deleteModal && (
          <DeleteProjectModal
            team={team}
            project={project}
            onClose={() => setDeleteModal(false)}
          />
        )}
        {lostAccessModal && (
          <LostAccessModal
            onClose={() => setLostAccessModal(false)}
            teamSlug={team?.slug || ""}
            projectSlug={slug}
          />
        )}
      </div>
    </Card>
  );
}

// Displays links to the production and development environment in the project card
//
// Uses Tailwind's group and peer hovering feature to underline the default (last viewed) environment when hovering the project card,
// but underline only the selected environment when hovering over a link.
//
// Because sibling elements with the `peer` classname need to be rendered first, we conditionally use `flex-row-reverse`
// to render the list backwards depending on which environment is the default (isProdDefault)
function DeploymentLinks({
  devHref,
  prodHref,
  isProdDefault,
  hasDefaultDevDeployment,
  hasDefaultProdDeployment,
}: {
  isProdDefault: boolean;
  devHref: string;
  prodHref: string;
  hasDefaultDevDeployment: boolean;
  hasDefaultProdDeployment: boolean;
}) {
  const prod = (
    <DeploymentLabel
      href={prodHref}
      isDefault={isProdDefault}
      title="Production"
      showTip={!hasDefaultProdDeployment}
      tip={
        <>
          You do not have a production deployment for this project yet. Click to
          provision one.
        </>
      }
    />
  );
  const dev = (
    <DeploymentLabel
      href={devHref}
      isDefault={!isProdDefault}
      showTip={!hasDefaultDevDeployment}
      tip={
        <>
          You do not have a personal development deployment for this project
          yet. Click to provision one, or run{" "}
          <code className="px-1">npx convex dev</code>.
        </>
      }
      title="Development"
    />
  );
  return (
    <div className="flex gap-1">
      <div
        className={`flex ${
          isProdDefault && "flex-row-reverse"
        } h-6 items-center justify-end gap-1 truncate text-xs`}
      >
        {isProdDefault ? dev : prod}
        <div className="text-neutral-4">•</div>
        {isProdDefault ? prod : dev}
      </div>
    </div>
  );
}

function DeploymentLabel({
  href,
  isDefault,
  title,
  tip,
  showTip,
}: {
  href: string;
  isDefault: boolean;
  title: string;
  tip?: ReactNode;
  showTip?: boolean;
}) {
  const linkContent = (
    <Link
      passHref
      href={href}
      className={`${
        isDefault ? "group-hover:underline peer-hover:no-underline" : "peer"
      } hover:underline`}
    >
      {title}
    </Link>
  );

  if (showTip && tip) {
    return <Tooltip tip={tip}>{linkContent}</Tooltip>;
  }

  return linkContent;
}
