import {
  ChevronUpIcon,
  ChevronDownIcon,
  SewingPinFilledIcon,
} from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { HTMLAttributeAnchorTarget, ReactNode } from "react";
import Link, { LinkProps } from "next/link";
import { cn } from "@ui/cn";
import { logEvent } from "convex-analytics";

export function SelectorItem({
  className,
  disabled,
  children,
  href,
  target,
  close,
  active = false,
  selected = false,
  onFocusOrMouseEnter,
  eventName,
}: {
  className?: string;
  disabled?: boolean;
  children: ReactNode;
  href: LinkProps["href"];
  target?: HTMLAttributeAnchorTarget;
  close: () => void;
  active?: boolean;
  selected?: boolean;
  onFocusOrMouseEnter?: () => void;
  eventName?: string;
}) {
  const fullClassName = cn(
    className,
    "SelectorItem flex w-full items-center text-sm",
    "rounded-sm p-2 text-left transition",
    "text-content-primary",
    "hover:bg-background-tertiary",
    active && "SelectorItem-active bg-background-tertiary",
    selected && "bg-background-tertiary/60",
    disabled === true
      ? "cursor-not-allowed text-content-tertiary"
      : "cursor-pointer",
  );

  return disabled ? (
    // eslint-disable-next-line react/forbid-elements
    <button type="button" className={fullClassName} disabled>
      {children}
    </button>
  ) : (
    <Link
      onMouseEnter={onFocusOrMouseEnter}
      onFocus={onFocusOrMouseEnter}
      href={href}
      className={fullClassName}
      onClick={() => {
        eventName && logEvent(eventName);
        close();
      }}
      target={target}
      role="menuitem"
    >
      {children}
      {selected && (
        <Tooltip tip="You are here." side="right" className="ml-auto">
          <SewingPinFilledIcon className="min-h-[1rem] min-w-[1rem]" />
        </Tooltip>
      )}
    </Link>
  );
}

export function selectorButtonComponent(
  selected: ReactNode | null,
  className?: string,
) {
  return function SelectorButton({ open }: { open: boolean }) {
    return (
      <Button
        variant="unstyled"
        type="button"
        className={cn(
          "h-10 rounded-sm outline-hidden focus-visible:ring-2 focus-visible:ring-util-accent",
          "flex w-fit items-center gap-2 px-3 py-2 select-none",
          ...(className !== undefined
            ? [className]
            : [
                "text-content-primary",
                "hover:bg-background-tertiary",
                open ? "bg-background-tertiary" : null,
              ]),
        )}
      >
        <div className="flex items-center gap-2 truncate text-sm select-none">
          {selected}
        </div>
        {open ? <ChevronUpIcon /> : <ChevronDownIcon />}
      </Button>
    );
  };
}
