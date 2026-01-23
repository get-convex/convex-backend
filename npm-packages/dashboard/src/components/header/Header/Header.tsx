import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import Image from "next/legacy/image";
import Link from "next/link";
import { SupportWidget, useSupportFormOpen } from "elements/SupportWidget";
import { Portal } from "@headlessui/react";
import { Button } from "@ui/Button";
import { AskAI } from "elements/AskAI";
import { DeploymentDisplay } from "elements/DeploymentDisplay";
import { useCurrentProject } from "api/projects";
import { User } from "@workos-inc/node";
import { ConvexStatusBadge } from "lib/ConvexStatusBadge";
import { useConvexStatus } from "hooks/useConvexStatus";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { UserMenu } from "../UserMenu/UserMenu";

type HeaderProps = {
  children?: React.ReactNode;
  logoLink?: string;
  user: User | null;
};

function ConvexStatus() {
  const { status } = useConvexStatus();

  // Only show if there are issues (not operational) or still loading
  if (!status || status.indicator === "none") {
    return null;
  }

  return (
    <div className="flex items-center px-2.5">
      <ConvexStatusBadge status={status} />
    </div>
  );
}

function Support() {
  const [openState, setOpenState] = useSupportFormOpen();
  return (
    <>
      <Button
        inline
        onClick={() => {
          setOpenState(!openState);
        }}
        type="button"
        className="flex items-center gap-1 rounded-full px-2.5 text-sm text-content-primary"
      >
        <QuestionMarkCircledIcon />
        <span className="hidden md:block">Support</span>
      </Button>
      <Portal>
        <SupportWidget />
      </Portal>
    </>
  );
}

export function Header({ children, logoLink = "/", user }: HeaderProps) {
  const project = useCurrentProject();
  const { enableStatuspageWidget } = useLaunchDarkly();

  return (
    <header
      className={classNames(
        "flex justify-between min-h-[56px] overflow-x-auto scrollbar-none bg-background-secondary border-b",
      )}
    >
      <div className="flex items-center bg-background-secondary px-2">
        <div className="rounded-full p-2 transition-colors hover:bg-background-tertiary">
          <Link
            href={logoLink}
            passHref
            className="flex min-h-[28px] min-w-[28px] rounded-full"
          >
            <Image
              src="/convex-logo-only.svg"
              width="28"
              height="28"
              alt="Convex logo"
            />
          </Link>
        </div>
        <div>{children}</div>
      </div>
      {project && <DeploymentDisplay project={project} />}
      <div className="flex items-center bg-background-secondary px-2">
        <div className="flex items-center">
          {enableStatuspageWidget && <ConvexStatus />}
          <AskAI />
          <Support />
        </div>
        {user && <UserMenu />}
      </div>
    </header>
  );
}
