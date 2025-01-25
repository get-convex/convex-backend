import { Menu, MenuLink, Tooltip, useTheme } from "dashboard-common";
import {
  GearIcon,
  SunIcon,
  MoonIcon,
  LightningBoltIcon,
  PersonIcon,
  ExitIcon,
} from "@radix-ui/react-icons";
import { useAuth0 } from "hooks/useAuth0";
import Image from "next/image";
import { useCurrentTeam } from "api/teams";
import { useCurrentProject } from "api/projects";
import { useProfile } from "api/profile";
import { cn } from "lib/cn";
import startCase from "lodash/startCase";
import { useRouter } from "next/router";

export function UserMenu() {
  const { user } = useAuth0();
  const profile = useProfile();
  const { theme: currentTheme, setTheme } = useTheme();
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const router = useRouter();
  const isAcceptingOptions = router.pathname.startsWith("/accept");
  return (
    <Menu
      buttonProps={{
        icon: user?.picture ? (
          <Image
            src={user.picture}
            priority
            alt="User profile image"
            width={32}
            height={32}
            className="min-h-[2rem] min-w-[2rem] rounded-full"
          />
        ) : (
          <GearIcon className="h-7 w-7 rounded p-1 text-content-primary hover:bg-background-tertiary" />
        ),
        variant: "unstyled",
        "aria-label": "User profile",
      }}
      placement="bottom-end"
    >
      {profile ? (
        <div className="flex min-w-[20rem] max-w-[20rem] flex-col gap-1 border-b px-3 pb-2">
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
      <div className="flex items-center justify-between px-3 py-1">
        <span className="select-none">Theme</span>
        <fieldset className="flex items-center rounded-full border">
          <ThemeRadioInput
            currentTheme={currentTheme}
            setTheme={setTheme}
            theme="system"
            className="rounded-l-full"
          />
          <ThemeRadioInput
            currentTheme={currentTheme}
            setTheme={setTheme}
            theme="light"
          />
          <ThemeRadioInput
            currentTheme={currentTheme}
            setTheme={setTheme}
            theme="dark"
            className="rounded-r-full"
          />
        </fieldset>
      </div>
      {team ? (
        <>
          <hr className="mx-4" />
          <Tooltip
            side="left"
            tip="Settings related to your team (e.g. billing, usage, and inviting team members)."
          >
            <MenuLink href="/team/settings" disabled={isAcceptingOptions}>
              <div className="flex w-full items-center justify-between">
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

function ThemeRadioInput({
  currentTheme,
  setTheme,
  className,
  theme,
}: {
  currentTheme?: string;
  setTheme: (theme: string) => void;
  className?: string;
  theme: string;
}) {
  const icon =
    theme === "light" ? (
      <SunIcon />
    ) : theme === "dark" ? (
      <MoonIcon />
    ) : (
      <LightningBoltIcon />
    );

  return (
    <>
      <input
        id={`${theme}-theme`}
        type="radio"
        onChange={() => setTheme(theme)}
        checked={!currentTheme || currentTheme === theme}
        hidden
      />
      <Tooltip tip={startCase(theme)} wrapsButton>
        <label
          aria-label="System Theme"
          htmlFor={`${theme}-theme`}
          className={cn(
            "p-1.5 cursor-pointer",
            currentTheme === theme
              ? "bg-util-accent text-white"
              : "hover:bg-background-tertiary",
            className,
          )}
        >
          {icon}
        </label>
      </Tooltip>
    </>
  );
}
