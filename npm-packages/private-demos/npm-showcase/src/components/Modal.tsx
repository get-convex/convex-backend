import { Fragment, ReactNode } from "react";
import { Dialog, Transition } from "@headlessui/react";
import { classNames } from "../utils";

type ModalProps = {
  onClose: () => void;
  title: string | ReactNode;
  description?: string | ReactNode;
  children: ReactNode;
  large?: boolean;
};

export default function Modal({
  onClose,
  title,
  description,
  children,
  large = false,
}: ModalProps) {
  return (
    <Transition.Root show as={Fragment} appear afterLeave={onClose}>
      <Dialog
        as="div"
        data-testid="modal"
        className="fixed inset-0 z-40 overflow-y-auto"
        onClose={onClose}
      >
        <div className="flex min-h-screen items-end justify-center px-4 pb-20 pt-4 text-center sm:block sm:p-0">
          <Transition.Child
            as={Fragment}
            enter="ease-out duration-300"
            enterFrom="opacity-0"
            enterTo="opacity-100"
            leave="ease-in duration-200"
            leaveFrom="opacity-100"
            leaveTo="opacity-0"
          >
            <Dialog.Overlay className="fixed inset-0 bg-neutral-8/75 transition-opacity" />
          </Transition.Child>

          {/* This element is to trick the browser into centering the modal contents. */}
          <span
            className="hidden sm:inline-block sm:h-screen sm:align-middle"
            aria-hidden="true"
          >
            &#8203;
          </span>
          <Transition.Child
            as={Fragment}
            enter="ease-out duration-300"
            enterFrom="opacity-0 translate-y-4 sm:translate-y-0 sm:scale-95"
            enterTo="opacity-100 translate-y-0 sm:scale-100"
            leave="ease-in duration-200"
            leaveFrom="opacity-100 translate-y-0 sm:scale-100"
            leaveTo="opacity-0 translate-y-4 sm:translate-y-0 sm:scale-95"
          >
            <div
              className={classNames(
                "inline-block bg-light-background-secondary dark:bg-dark-background-secondary rounded",
                "text-light-content-primary dark:text-dark-content-primary",
                "text-left overflow-hidden shadow-4 dark:border transform",
                "transition-all align-middle",
                large ? "sm:max-w-6xl" : "sm:max-w-xl",
                "w-full",
              )}
            >
              {/* Header */}
              <div className="px-4 pt-6 sm:px-6">
                <div className="flex items-start justify-between">
                  <div className="mb-2">
                    <Dialog.Title className="text-lg font-semibold">
                      {title}
                    </Dialog.Title>
                    <Dialog.Title className="mt-1 text-xs">
                      {description}
                    </Dialog.Title>
                  </div>
                </div>
              </div>

              {/* Contents */}
              <div className="mx-6 mb-6">{children}</div>
            </div>
          </Transition.Child>
        </div>
      </Dialog>
    </Transition.Root>
  );
}
