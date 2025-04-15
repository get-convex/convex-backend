import { Fragment, ReactNode, useState } from "react";
import { Menu as HeadlessMenu, Portal } from "@headlessui/react";
import { PopperChildrenProps, usePopper } from "react-popper";
import classNames from "classnames";
import { Button, ButtonProps } from "@ui/Button";
import { Key, KeyboardShortcut } from "@ui/KeyboardShortcut";
import { TooltipSide } from "@ui/Tooltip";

export type MenuProps = {
  children: React.ReactElement | (React.ReactElement | null)[];
  buttonProps: ButtonProps;
  placement?: PopperChildrenProps["placement"];
  offset?: number;
};

export function Menu({
  children,
  buttonProps,
  placement = "bottom",
  offset = 4,
}: MenuProps) {
  const [referenceElement, setReferenceElement] =
    useState<HTMLButtonElement | null>(null);
  const [popperElement, setPopperElement] = useState<HTMLDivElement | null>();
  const { styles, attributes } = usePopper(referenceElement, popperElement, {
    placement,
    modifiers: offset
      ? [
          {
            name: "offset",
            options: { offset: [0, offset] },
          },
        ]
      : [],
  });

  return (
    <HeadlessMenu>
      {({ open }) => (
        <>
          <HeadlessMenu.Button
            ref={setReferenceElement}
            as={Fragment}
            data-testid="open-menu"
          >
            <Button {...buttonProps} focused={open} />
          </HeadlessMenu.Button>
          <Portal>
            <HeadlessMenu.Items
              ref={setPopperElement}
              style={styles.popper}
              {...attributes.popper}
              className="z-50 flex max-h-[20rem] flex-col gap-1 overflow-auto whitespace-nowrap rounded-lg border bg-background-secondary py-2 text-sm shadow-md"
            >
              {children}
            </HeadlessMenu.Items>
          </Portal>
        </>
      )}
    </HeadlessMenu>
  );
}

export function MenuItem({
  variant = "default",
  children,
  action,
  href,
  disabled = false,
  tip,
  tipSide,
  shortcut,
}: {
  variant?: "default" | "danger";
  children: ReactNode;
  action?: () => void;
  href?: string;
  disabled?: boolean;
  tip?: ReactNode;
  tipSide?: TooltipSide;
  shortcut?: Key[];
}) {
  return (
    <HeadlessMenu.Item>
      {({ active }) => (
        <Button
          tip={tip}
          tipSide={tipSide}
          variant="unstyled"
          className={classNames(
            "mx-1 flex gap-2 items-center p-2 rounded-sm",
            disabled
              ? "cursor-not-allowed fill-content-tertiary text-content-tertiary"
              : "hover:bg-background-tertiary",
            active && "bg-background-primary",
            !disabled && variant === "danger"
              ? "text-content-errorSecondary"
              : "text-content-primary",
          )}
          disabled={disabled}
          onClick={action}
          href={href}
        >
          {children}
          {shortcut && (
            <KeyboardShortcut
              value={shortcut}
              className="ml-auto pl-6 text-content-tertiary"
            />
          )}
        </Button>
      )}
    </HeadlessMenu.Item>
  );
}

export function MenuLink({
  children,
  href,
  disabled = false,
  selected = false,
  shortcut,
}: {
  children: React.ReactChild | React.ReactChild[];
  href: string;
  disabled?: boolean;
  selected?: boolean;
  shortcut?: Key[];
}) {
  return (
    <HeadlessMenu.Item disabled={disabled}>
      {({ active, close }) => (
        <a
          href={href}
          aria-disabled={disabled}
          onClick={disabled ? (e) => e.preventDefault() : () => close()}
          className={classNames(
            "rounded-sm flex gap-2 items-center mx-1 px-2 py-2 text-content-primary",
            disabled &&
              "cursor-not-allowed fill-content-secondary bg-background-tertiary text-content-secondary",

            active || selected
              ? "bg-background-primary"
              : "hover:bg-background-tertiary",
          )}
        >
          {children}
          {shortcut && (
            <KeyboardShortcut
              value={shortcut}
              className="ml-auto pl-6 text-content-tertiary"
            />
          )}
        </a>
      )}
    </HeadlessMenu.Item>
  );
}
