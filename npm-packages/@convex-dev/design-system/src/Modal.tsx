import React, { Fragment, ReactNode, useState } from "react";
import { Dialog, Transition } from "@headlessui/react";
import classNames from "classnames";
import { ClosePanelButton } from "@ui/ClosePanelButton";

type ModalProps = {
  onClose: () => void;
  title: string | ReactNode;
  description?: string | ReactNode;
  children: ReactNode;
  size?: "sm" | "md" | "lg";
};

export function Modal({
  onClose,
  title,
  description,
  children,
  size = "sm",
}: ModalProps) {
  const [open, setOpen] = useState(true);
  const handleClose = () => {
    setOpen(false);
  };
  return (
    <Transition.Root as={Fragment} appear afterLeave={onClose} show={open}>
      <Dialog
        as="div"
        data-testid="modal"
        className="fixed inset-0 z-40 overflow-hidden"
        onClose={handleClose}
      >
        <div className="flex sm:min-h-screen sm:items-center sm:justify-center sm:px-4">
          <Transition.Child
            as={Fragment}
            enter="ease-out duration-300"
            enterFrom="opacity-0"
            enterTo="opacity-100"
            leave="ease-in duration-200"
            leaveFrom="opacity-100"
            leaveTo="opacity-0"
          >
            <Dialog.Overlay className="fixed inset-0 bg-black/50 transition-opacity" />
          </Transition.Child>

          <Transition.Child
            as={Fragment}
            enter="ease-out duration-300"
            enterFrom="opacity-0 translate-y-12"
            enterTo="opacity-100 translate-y-0"
            leave="ease-in duration-200"
            leaveFrom="opacity-100 translate-y-0"
            leaveTo="opacity-0 translate-y-12"
          >
            <div
              className={classNames(
                "inline-block bg-background-secondary rounded-xl",
                "text-content-primary",
                "text-left shadow-xl dark:border transform",
                "transition-all align-middle",
                "rounded-b-none sm:rounded-b-xl",
                "absolute bottom-0 sm:relative",
                size === "lg"
                  ? "sm:max-w-6xl"
                  : size === "md"
                    ? "sm:max-w-3xl"
                    : "sm:max-w-xl",
                "w-full",
              )}
            >
              {/* Header */}
              <div className="p-6 pb-2">
                <div className="flex items-start justify-between">
                  <div>
                    <Dialog.Title as="h4">{title}</Dialog.Title>
                    <Dialog.Description className="mt-1 text-sm">
                      {description}
                    </Dialog.Description>
                  </div>
                  <ClosePanelButton onClose={handleClose} />
                </div>
              </div>

              {/* Contents */}
              <div className="mx-6 mb-12 max-h-[80dvh] overflow-y-auto sm:mb-6">
                {children}
              </div>
            </div>
          </Transition.Child>
        </div>
      </Dialog>
    </Transition.Root>
  );
}
