import { User } from "hooks/useAuth0";
import classNames from "classnames";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";
import Image from "next/legacy/image";
import Link from "next/link";
import { SupportWidget, useSupportFormOpen } from "elements/SupportWidget";
import { Portal } from "@headlessui/react";
import { Button } from "@ui/Button";
import { useReducer } from "react";
import { AskAI } from "elements/AskAI";
import { UserMenu } from "../UserMenu/UserMenu";

type HeaderProps = {
  children?: React.ReactNode;
  logoLink?: string;
  user?: User;
};

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
        className="flex items-center gap-1 px-2.5 text-sm text-content-primary"
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
  const [headerKey, forceRerender] = useReducer((x) => x + 1, 0);
  return (
    <header
      className={classNames(
        "border-b flex gap-1 sm:gap-6 justify-between px-4 min-h-[56px] bg-background-secondary overflow-x-auto scrollbar-none",
      )}
      // Re-render the header content when the user scrolls so
      // the underline on the active nav item can be updated.
      // TODO: Don't absolutely position the underline
      onScroll={forceRerender}
    >
      <div className="flex items-center gap-2">
        <Link
          href={logoLink}
          passHref
          className="flex min-h-[28px] min-w-[28px]"
        >
          <Image
            src="/convex-logo-only.svg"
            width="28"
            height="28"
            alt="Convex logo"
          />
        </Link>
        <div key={headerKey}>{children}</div>
      </div>
      <div className="flex items-center gap-2">
        <div className="flex">
          <AskAI />
          <Support />
        </div>
        {user && <UserMenu />}
      </div>
    </header>
  );
}
