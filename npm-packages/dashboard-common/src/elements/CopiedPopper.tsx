import { usePopper } from "react-popper";
import { Transition } from "@headlessui/react";
import { CheckCircledIcon } from "@radix-ui/react-icons";

type CopiedPopperProps = {
  referenceElement: HTMLElement | null;
  copiedPopperElement: HTMLDivElement | null;
  setCopiedPopperElement: (element: HTMLDivElement | null) => void;
  show: boolean;
  message?: string;
  placement?:
    | "bottom-start"
    | "bottom"
    | "bottom-end"
    | "top-start"
    | "top"
    | "top-end";
  offset?: [number, number];
};

export function CopiedPopper({
  referenceElement,
  copiedPopperElement,
  setCopiedPopperElement,
  show,
  message = "Copied",
  placement = "bottom-start",
  offset = [0, 4],
}: CopiedPopperProps) {
  const { styles, attributes } = usePopper(
    referenceElement,
    copiedPopperElement,
    {
      placement,
      modifiers: [
        {
          name: "offset",
          options: { offset },
        },
      ],
    },
  );

  return (
    <Transition
      show={show}
      enter="transition-opacity ease-in-out duration-100"
      enterFrom="opacity-0"
      enterTo="opacity-100"
      leave="transition-opacity ease-in-out duration-100"
      leaveFrom="opacity-100"
      leaveTo="opacity-0"
    >
      <div
        ref={setCopiedPopperElement}
        style={styles.popper}
        className="z-50 flex items-center gap-1 rounded-sm border bg-background-tertiary p-1 text-xs"
        data-testid="copied-popper"
        {...attributes.popper}
      >
        <CheckCircledIcon />
        {message}
      </div>
    </Transition>
  );
}
