import React, { MutableRefObject, useEffect, useState } from "react";
import { PopperChildrenProps, usePopper } from "react-popper";
import {
  Popover as HeadlessPopover,
  PopoverPanel as HeadlessPopoverPanel,
  PopoverButton as HeadlessPopoverButton,
  Portal,
} from "@headlessui/react";
import classNames from "classnames";

// Copied from HeadlessUI Types
type FunctionalChild = (bag: {
  open: boolean;
  close: (
    focusableElement?: HTMLElement | MutableRefObject<HTMLElement | null>,
  ) => void;
}) => React.ReactElement<any, string | React.JSXElementConstructor<any>>;

type PopoverProps = {
  children: React.ReactNode | FunctionalChild;
  className?: string;
  openButtonClassName?: string;
  button: React.ReactNode | FunctionalChild;
  placement?: PopperChildrenProps["placement"];
  offset?: [number | null | undefined, number | null | undefined];
  onOpen?(): void;
  onClose?(): void;
  // If true, will render in a Portal
  portal?: boolean;
  padding?: boolean;
  focus?: boolean;
};

export function Popover({
  className,
  openButtonClassName = "",
  children,
  button,
  placement = "bottom",
  offset = [0, 8],
  onOpen,
  onClose,
  portal,
  padding = true,
  focus = false,
}: PopoverProps) {
  const [referenceElement, setReferenceElement] =
    useState<HTMLButtonElement | null>(null);
  const [popperElement, setPopperElement] = useState<HTMLElement | null>();
  const { styles, attributes } = usePopper(referenceElement, popperElement, {
    placement,
    modifiers: [
      {
        name: "offset",
        options: { offset },
      },
    ],
  });

  useEffect(() => {
    const isOpen = !!popperElement;
    const fn = isOpen ? onOpen : onClose;
    fn && fn();
  }, [popperElement, onOpen, onClose]);

  return (
    <HeadlessPopover>
      {({ open }) => {
        const panel = (
          <HeadlessPopoverPanel
            ref={setPopperElement}
            style={styles.popper}
            {...attributes.popper}
            focus={focus}
            className={classNames(
              "z-50 bg-background-secondary shadow-md border rounded-lg",
              padding && "py-4 px-5",
              className,
            )}
          >
            {children}
          </HeadlessPopoverPanel>
        );
        return (
          <>
            <HeadlessPopoverButton
              ref={setReferenceElement}
              as="div"
              className={open ? openButtonClassName : ""}
            >
              {button as any /* TODO(react-18-upgrade) */}
            </HeadlessPopoverButton>
            {portal ? <Portal>{panel}</Portal> : panel}
          </>
        );
      }}
    </HeadlessPopover>
  );
}
