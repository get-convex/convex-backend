import { Menu, MenuLink } from "@ui/Menu";
import { Tooltip } from "@ui/Tooltip";
import { ToggleTheme } from "@common/elements/ToggleTheme";
import { GearIcon, PersonIcon, ExitIcon } from "@radix-ui/react-icons";
import { useWorkOS } from "hooks/useWorkOS";
import Image from "next/image";
import { useCurrentTeam } from "api/teams";
import { useCurrentProject } from "api/projects";
import { useProfile } from "api/profile";
import { useRouter } from "next/router";

export function UserMenu() {
  const { user } = useWorkOS();
  const profile = useProfile();
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const router = useRouter();
  const isAcceptingOptions = router.pathname.startsWith("/accept");
  return (
    <Menu
      buttonProps={{
        icon: user?.profilePictureUrl ? (
          <Image
            src={user.profilePictureUrl}
            priority
            alt="User profile image"
            width={32}
            height={32}
            className="min-h-[2rem] min-w-[2rem] rounded-full"
          />
        ) : (
          <GearIcon className="h-7 w-7 rounded-sm p-1 text-content-primary hover:bg-background-tertiary" />
        ),
        variant: "unstyled",
        className:
          "rounded-full p-2 transition-colors hover:bg-background-tertiary",
        "aria-label": "User profile",
      }}
      placement="bottom-end"
    >
      {profile ? (
        <div className="flex max-w-[20rem] min-w-[20rem] flex-col gap-1 border-b px-3 pb-2">
          {profile.name && (
            <div className="text-sm font-semibold text-content-primary">
              {profile.name}
            </div>
          )}
          <div
            className={`${
              profile.name
                ? "text-xs text-content-secondary"
                : "text-sm text-content-primary"
            }`}
          >
            {profile.email}
          </div>
        </div>
      ) : null}
      <Tooltip
        side="left"
        tip="Settings related to your personal profile (e.g. name and email)."
      >
        <MenuLink href="/profile" disabled={isAcceptingOptions}>
          <div className="flex w-full items-center justify-between">
            Profile Settings
            <PersonIcon className="text-content-secondary" />
          </div>
        </MenuLink>
      </Tooltip>
      <ToggleTheme />
      {team ? (
        <>
          <hr className="mx-4" />
          <Tooltip
            side="left"
            tip="Settings related to your team (e.g. billing, usage, and inviting team members)."
          >
            <MenuLink href="/team/settings" disabled={isAcceptingOptions}>
              <div className="flex w-full items-center justify-between gap-1">
                Team Settings
                <span className="max-w-[6rem] truncate text-xs text-content-secondary">
                  {team.name}
                </span>
              </div>
            </MenuLink>
          </Tooltip>

          {project ? (
            <Tooltip
              side="left"
              tip="Settings related to your project (e.g. name, slug, and access controls)."
            >
              <MenuLink href={`/t/${team.slug}/${project.slug}/settings`}>
                <div className="flex w-full items-center justify-between">
                  Project Settings
                  <span className="max-w-[6rem] truncate text-xs text-content-secondary">
                    {project.name}
                  </span>
                </div>
              </MenuLink>
            </Tooltip>
          ) : null}
        </>
      ) : null}
      <hr className="mx-4" />
      <MenuLink href="/api/auth/logout">
        <div className="flex w-full items-center justify-between">
          Log Out
          <ExitIcon className="text-content-secondary" />
        </div>
      </MenuLink>
    </Menu>
  );
}
