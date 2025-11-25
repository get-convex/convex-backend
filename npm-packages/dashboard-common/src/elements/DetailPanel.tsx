import {
  Dialog,
  DialogPanel,
  DialogTitle,
  Transition,
  TransitionChild,
} from "@headlessui/react";
import { ReactNode, useCallback, useState } from "react";
import { Loading } from "@ui/Loading";
import { Callout } from "@ui/Callout";
import { ClosePanelButton } from "@ui/ClosePanelButton";

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
  const closePanel = useCallback(() => {
    setOpen(false);
  }, [setOpen]);

  let detailContent;
  if (error) {
    detailContent = <Callout variant="error">{error}</Callout>;
  } else if (content !== undefined) {
    detailContent = content;
  } else {
    detailContent = <Loading />;
  }

  return (
    <Transition show={open} appear afterLeave={onClose}>
      <Dialog
        static
        as="div"
        className="fixed inset-0 z-50 overflow-hidden"
        open // Real openness status is controlled by Transition above
        onClose={closePanel}
      >
        <div className="absolute inset-0 overflow-hidden">
          <TransitionChild
            enter="ease-in-out duration-300"
            enterFrom="opacity-0"
            enterTo="opacity-100"
            leave="ease-in-out duration-300"
            leaveFrom="opacity-100"
            leaveTo="opacity-0"
          >
            <div className="absolute inset-0 transition-opacity" />
          </TransitionChild>

          <div className="fixed inset-y-0 right-0 flex max-w-full pl-10">
            <TransitionChild
              enter="transform transition ease-in-out duration-200 sm:duration-300"
              enterFrom="translate-x-full"
              enterTo="translate-x-0"
              leave="transform transition ease-in-out duration-200 sm:duration-300"
              leaveFrom="translate-x-0"
              leaveTo="translate-x-full"
            >
              <DialogPanel className="w-screen max-w-2xl">
                <div className="flex h-full flex-col bg-background-secondary shadow-xl dark:border">
                  {/* Header */}
                  <div className="px-4 pt-6 sm:px-6">
                    <div className="flex items-center justify-between">
                      <DialogTitle as="h4">{header}</DialogTitle>
                      <ClosePanelButton onClose={closePanel} />
                    </div>
                  </div>

                  <div className="relative flex-1 px-4 py-6 sm:px-6">
                    <div className="absolute inset-6">{detailContent}</div>
                  </div>
                </div>
              </DialogPanel>
            </TransitionChild>
          </div>
        </div>
      </Dialog>
    </Transition>
  );
}
