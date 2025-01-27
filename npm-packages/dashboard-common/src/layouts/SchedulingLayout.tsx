import { Tab as HeadlessTab } from "@headlessui/react";
import { useRouter } from "next/router";
import { NentSwitcher } from "../elements/NentSwitcher";
import { Tab } from "../elements/Tab";

export function SchedulingLayout({ children }: { children: React.ReactNode }) {
  const { pathname, query } = useRouter();
  // The current path without the last part
  const pathParts = pathname.split("/");
  const basePath = pathParts.slice(0, -1).join("/");
  const currentPage = pathParts[pathParts.length - 1];

  return (
    <div className="flex h-full flex-col p-6 py-4">
      <div className="w-fit min-w-60">
        <NentSwitcher />
      </div>
      <div className="-ml-2 mb-4 flex gap-4">
        <HeadlessTab.Group
          selectedIndex={currentPage.startsWith("functions") ? 0 : 1}
        >
          <Tab
            large
            href={{
              pathname: `${basePath}/functions`,
              query,
            }}
          >
            Scheduled Functions
          </Tab>
          <Tab
            large
            href={{
              pathname: `${basePath}/crons`,
              query,
            }}
          >
            Cron Jobs
          </Tab>
        </HeadlessTab.Group>
      </div>
      <div className="grow">{children}</div>
    </div>
  );
}
