import { LinkBreak2Icon } from "@radix-ui/react-icons";
import { Sheet } from "@ui/Sheet";
import { ReactNode } from "react";

export function DisconnectedOverlay({ children }: { children: ReactNode }) {
  return (
    <div className="absolute z-50 mt-[3.5rem] flex h-[calc(100vh-3.5rem)] w-full items-center justify-center backdrop-blur-[4px]">
      <Sheet className="scrollbar flex max-h-[80vh] max-w-[28rem] animate-fadeInFromLoading flex-col items-start gap-2 overflow-y-auto rounded-xl bg-background-secondary/90 backdrop-blur-[8px]">
        <h3 className="mb-4 flex items-center gap-3">
          <div className="flex aspect-square h-[2.625rem] shrink-0 items-center justify-center rounded-lg border bg-gradient-to-tr from-yellow-200 to-util-brand-yellow text-yellow-900 shadow-md">
            <LinkBreak2Icon className="size-6" />
          </div>
          Connection Issue
        </h3>
        {children}
      </Sheet>
    </div>
  );
}
