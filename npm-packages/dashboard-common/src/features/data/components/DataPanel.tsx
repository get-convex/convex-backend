import { ReactNode } from "react";
import { ClosePanelButton } from "@common/elements/ClosePanelButton";
import { Panel } from "react-resizable-panels";
import { ResizeHandle } from "@common/layouts/SidebarDetailLayout";

export interface DataPanelProps {
  title: ReactNode;
  onClose: () => void;
  children: ReactNode;
  "data-testid"?: string;
}

export function DataPanel({
  title,
  onClose,
  children,
  ...props
}: DataPanelProps) {
  return (
    <>
      <ResizeHandle direction="left" collapsed={false} className="ml-6" />
      <Panel
        className="z-40 flex h-full min-w-[14rem] max-w-[42rem] shrink overflow-x-auto"
        defaultSize={20}
      >
        <div
          className="w-full bg-background-secondary shadow-xl dark:border-l"
          {...props}
        >
          <div className="flex h-full max-h-full flex-col">
            <div className="mb-1 px-4 pt-6 sm:px-6">
              <div className="flex flex-wrap items-center justify-between gap-4 gap-y-2">
                <h4 className="flex-1 break-words">{title}</h4>
                <ClosePanelButton onClose={onClose} className="ml-auto" />
              </div>
            </div>
            {children}
          </div>
        </div>
      </Panel>
    </>
  );
}
