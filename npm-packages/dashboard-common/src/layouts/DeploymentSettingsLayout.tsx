import React, { ReactNode, useContext, useRef } from "react";
import { HamburgerMenuIcon } from "@radix-ui/react-icons";
import { useMeasure } from "react-use";
import {
  SettingsSidebar,
  SettingsPageKind,
} from "@common/layouts/SettingsSidebar";
import { PageContent } from "@common/elements/PageContent";
import { Popover } from "@ui/Popover";
import { Button } from "@ui/Button";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useIsOverflowing } from "@common/lib/useIsOverflowing";
import { Tooltip } from "@ui/Tooltip";

export function DeploymentSettingsLayout({
  page,
  children,
}: {
  page: SettingsPageKind;
  children: ReactNode;
}) {
  const sidebar = <SettingsSidebar selectedPage={page} />;
  const [ref, { width }] = useMeasure<HTMLDivElement>();
  const isWide = width > 700;

  return (
    <PageContent>
      <div
        className="flex h-full max-h-full w-full flex-col overflow-y-hidden"
        ref={ref}
      >
        {isWide ? (
          <SettingsMenuHeader />
        ) : (
          <Popover
            placement="bottom-start"
            className="bg-background-secondary"
            offset={[0, -4]}
            button={<SettingsMenuButton open={false} />}
          >
            {sidebar}
          </Popover>
        )}
        {/* Make space for the header above */}
        <div className="flex h-full w-full overflow-y-hidden">
          {isWide && sidebar}
          <div className="scrollbar flex w-full min-w-[22rem] grow overflow-auto">
            <div className="flex h-fit grow flex-col gap-6 p-6 sm:max-w-[65rem]">
              {children}
            </div>
          </div>
        </div>
      </div>
    </PageContent>
  );
}

function SettingsMenuHeader({ isMenu = false }: { isMenu?: boolean }) {
  return (
    <h2 className="flex w-full items-center gap-2 border-b bg-background-secondary p-4">
      {isMenu ? <HamburgerMenuIcon className="mt-0.5 min-w-[1rem]" /> : null}
      <span className="truncate">
        <DeploymentSettingsText />
      </span>
    </h2>
  );
}

function SettingsMenuButton({ open }: { open: boolean }) {
  return (
    <Button
      inline
      focused={open}
      variant="unstyled"
      size="sm"
      className="w-full"
    >
      <SettingsMenuHeader isMenu />
    </Button>
  );
}

function DeploymentSettingsText() {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  const ref = useRef<HTMLDivElement>(null);
  const isOverflowing = useIsOverflowing(ref);
  if (deployment === undefined) {
    return <>Deployment Settings</>;
  }
  switch (deployment.deploymentType) {
    case "prod":
      return <>Production Deployment Settings</>;
    case "dev":
      return <>Personal Deployment Settings</>;
    case "preview":
      if (deployment.previewIdentifier !== null) {
        return (
          <Tooltip
            tip={
              isOverflowing ? (
                <div className="break-all">{deployment.previewIdentifier}</div>
              ) : undefined
            }
          >
            <div className="flex items-baseline gap-2">
              <code className="max-w-md truncate" ref={ref}>
                {deployment.previewIdentifier}
              </code>{" "}
              Deployment Settings
            </div>
          </Tooltip>
        );
      }
      return <>Preview Deployment Settings</>;
    default: {
      deployment.deploymentType satisfies never;
      throw new Error("Unknown deployment type");
    }
  }
}
