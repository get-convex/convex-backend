/* This example requires Tailwind CSS v2.0+ */
import { Dialog, Transition } from "@headlessui/react";
import { Fragment, ReactNode, useState } from "react";
import { Loading } from "elements/Loading";
import { Callout } from "elements/Callout";
import { ClosePanelButton } from "elements/ClosePanelButton";

export function DetailPanel({
  onClose,
  content,
  header,
  error,
}: {
  onClose: () => void;
  content: string | any;
  header?: string | ReactNode;
  error?: string | ReactNode;
}) {
  const [open, setOpen] = useState(true);

  const closePanel = () => {
    setOpen(false);
  };
  let detailContent;
  if (error) {
    detailContent = <Callout variant="error">{error}</Callout>;
  } else if (content !== undefined) {
    detailContent = content;
  } else {
    detailContent = <Loading />;
  }
  return (
    <Transition.Root show={open} as={Fragment} appear afterLeave={onClose}>
      <Dialog
        as="div"
        className="fixed inset-0 z-50 overflow-hidden"
        onClose={closePanel}
      >
        <div className="absolute inset-0 overflow-hidden">
          <Transition.Child
            as={Fragment}
            enter="ease-in-out duration-300"
            enterFrom="opacity-0"
            enterTo="opacity-100"
            leave="ease-in-out duration-300"
            leaveFrom="opacity-100"
            leaveTo="opacity-0"
          >
            <Dialog.Overlay className="absolute inset-0 transition-opacity" />
          </Transition.Child>

          <div className="fixed inset-y-0 right-0 flex max-w-full pl-10">
            <Transition.Child
              as={Fragment}
              enter="transform transition ease-in-out duration-200 sm:duration-300"
              enterFrom="translate-x-full"
              enterTo="translate-x-0"
              leave="transform transition ease-in-out duration-200 sm:duration-300"
              leaveFrom="translate-x-0"
              leaveTo="translate-x-full"
            >
              <div className="w-screen max-w-2xl">
                <div className="flex h-full flex-col bg-background-secondary shadow-xl dark:border">
                  {/* Header */}
                  <div className="px-4 pt-6 sm:px-6">
                    <div className="flex items-center justify-between">
                      <Dialog.Title as="h4">{header}</Dialog.Title>
                      <ClosePanelButton onClose={closePanel} />
                    </div>
                  </div>

                  <div className="relative flex-1 px-4 py-6 sm:px-6">
                    <div className="absolute inset-6 ">{detailContent}</div>
                  </div>
                </div>
              </div>
            </Transition.Child>
          </div>
        </div>
      </Dialog>
    </Transition.Root>
  );
}
