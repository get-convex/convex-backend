import {
  Cross2Icon,
  ExclamationTriangleIcon,
  InfoCircledIcon,
} from "@radix-ui/react-icons";
import classNames from "classnames";
import { Button, buttonClasses } from "dashboard-common/elements/Button";
import { useUnpauseTeam } from "api/teams";
import { useTeamUsageState } from "hooks/useTeamUsageState";
import Link from "next/link";
import { useEffect, useState } from "react";
import { Team } from "generatedApi";

export type Variant = "Approaching" | "Exceeded" | "Disabled" | "Paused";

export function useCurrentUsageBanner(teamId: number | null): Variant | null {
  const { isDismissed } = useDismiss(teamId);

  const currentVariant = useTeamUsageState(teamId);
  if (
    !currentVariant ||
    currentVariant === "Default" ||
    (isDismissable(currentVariant) && isDismissed)
  ) {
    return null;
  }
  return currentVariant;
}

export function UsageBanner({
  variant,
  team,
}: {
  variant: Variant;
  team: Team;
}) {
  const { dismiss } = useDismiss(team.id);

  const {
    title,
    containerClass,
    primaryButtonClass,
    secondaryButtonClass,
    icon: Icon,
  } = getVariantDetails(variant);

  const primaryButtonClassFull = classNames(
    buttonClasses({
      variant: variant === "Approaching" ? "primary" : "unstyled",
      size: "sm",
    }),
    primaryButtonClass,
    "px-2.5 py-2 rounded text-sm font-medium",
    "ml-2",
  );
  const secondaryButtonClassFull = classNames(
    buttonClasses({
      variant: "unstyled",
      size: "sm",
    }),
    "hover:opacity-75",
    "px-1 py-2 rounded text-xs font-medium",
    secondaryButtonClass,
  );

  const unpauseTeam = useUnpauseTeam(team.id);
  const [isRestoringTeam, setIsRestoringTeam] = useState(false);

  return (
    <div
      className={classNames(
        "grid grid-cols-[auto_1fr] sm:flex shrink-0 sm:h-12 h-24 items-center px-2 py-1 border-b gap-2 overflow-x-hidden",
        containerClass,
      )}
    >
      <Icon className="h-4 w-4" />

      <div className="flex min-w-[12em] flex-1 items-center gap-1 text-xs">
        {title}
      </div>

      <div className="col-span-2 flex items-center justify-end">
        {variant !== "Paused" && (
          <>
            <Link
              className={secondaryButtonClassFull}
              href={`/${team.slug}/settings/usage`}
            >
              View Usage
            </Link>

            <Link
              className={primaryButtonClassFull}
              href={`/${team.slug}/settings/billing`}
            >
              Upgrade
            </Link>
          </>
        )}

        {variant === "Paused" && (
          <Button
            variant="unstyled"
            className={classNames(
              primaryButtonClassFull,
              "disabled:opacity-50 disabled:pointer-events-none",
            )}
            disabled={isRestoringTeam}
            onClick={async () => {
              setIsRestoringTeam(true);
              void unpauseTeam();
            }}
          >
            Enable All Projects
          </Button>
        )}

        {isDismissable(variant) && (
          <Button
            // eslint-disable-next-line tailwindcss/migration-from-tailwind-2
            className="ml-2 h-fit hover:bg-opacity-50"
            variant="neutral"
            size="xs"
            inline
            title="Dismiss"
            onClick={dismiss}
          >
            <Cross2Icon />
          </Button>
        )}
      </div>
    </div>
  );
}

function isDismissable(variant: Variant) {
  return variant === "Approaching";
}

function getVariantDetails(variant: Variant): {
  title: string;
  containerClass: string;
  primaryButtonClass: string;
  secondaryButtonClass: string;
  icon: React.FC<{ className: string | undefined }>;
} {
  const dangerStyle = {
    // eslint-disable-next-line no-restricted-syntax
    containerClass: "bg-red-700 text-white",
    // eslint-disable-next-line no-restricted-syntax
    primaryButtonClass: "bg-red-100 text-red-900 hover:bg-red-300",
    secondaryButtonClass: "text-white",
    icon: ExclamationTriangleIcon,
  };

  switch (variant) {
    case "Approaching":
      return {
        title:
          "Your projects are approaching the Starter plan limits. Consider upgrading to avoid service interruption.",
        containerClass: "bg-blue-100 dark:bg-blue-900",
        primaryButtonClass: "",
        secondaryButtonClass: "text-blue-900 text-content-primary",
        icon: InfoCircledIcon,
      };
    case "Exceeded":
      return {
        title:
          "Your projects are above the Starter plan limits. Decrease your usage or upgrade to avoid service interruption.",
        containerClass: "bg-background-warning dark:text-white",
        primaryButtonClass:
          "bg-yellow-500 text-black hover:bg-yellow-700 hover:text-white",
        secondaryButtonClass: "text-cyan-900 dark:text-white",
        icon: ExclamationTriangleIcon,
      };
    case "Disabled":
      return {
        title:
          "Your projects are disabled because the team exceeded Starter plan limits. Decrease your usage or upgrade to reenable your projects.",
        ...dangerStyle,
      };
    case "Paused":
      return {
        title:
          // This is shown as disabled to the user to not confuse them
          "Your projects are disabled because the team previously exceeded Starter plan limits.",
        ...dangerStyle,
      };
    default:
      throw new Error("Unexpected variant");
  }
}

function useDismiss(teamId: number | null) {
  const key = `usage-banner-dismissed-${teamId}`;

  const [isDismissed, setIsDismissed] = useState(true);

  useEffect(() => {
    if (teamId === null) {
      return undefined;
    }

    // Load the value from localStorage when the component mounts
    setIsDismissed(localStorage.getItem(key) !== null);

    // Get updates from other components that are also using useDismiss
    const listener = () => setIsDismissed(true);
    window.addEventListener(key, listener);
    return () => window.removeEventListener(key, listener);
  }, [teamId, key]);

  return {
    isDismissed,
    dismiss() {
      if (teamId !== null) {
        setIsDismissed(true);
        window.dispatchEvent(new Event(key));

        localStorage.setItem(`usage-banner-dismissed-${teamId}`, "true");
      }
    },
  };
}
