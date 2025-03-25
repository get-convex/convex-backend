import classNames from "classnames";
import { Loading } from "dashboard-common/elements/Loading";
import { PageContent } from "dashboard-common/elements/PageContent";
import { useCurrentTeam, useTeamEntitlements } from "api/teams";
import startCase from "lodash/startCase";
import Head from "next/head";
import React from "react";
import { Team } from "generatedApi";
import { SidebarLink } from "dashboard-common/elements/Sidebar";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";

export function TeamSettingsLayout({
  page: selectedPage,
  Component,
  title,
}: {
  page:
    | "general"
    | "members"
    | "billing"
    | "usage"
    | "audit-log"
    | "access-tokens"
    | "referrals";
  Component: React.FunctionComponent<{ team: Team }>;
  title: string;
}) {
  const selectedTeam = useCurrentTeam();
  const { referralsPage } = useLaunchDarkly();

  const auditLogsEnabled = useTeamEntitlements(
    selectedTeam?.id,
  )?.auditLogsEnabled;

  const pages = [
    "general",
    "members",
    "billing",
    "usage",
    ...(referralsPage ? ["referrals"] : []),
  ];

  return (
    <>
      <Head>
        {selectedTeam && (
          <title>
            {title} | {selectedTeam.name} | Convex Dashboard
          </title>
        )}
      </Head>
      <PageContent>
        <div
          className={classNames(
            "flex flex-col sm:flex-row h-full",
            "oveflow-hidden",
          )}
        >
          <aside
            className={classNames(
              "flex sm:flex-col gap-1",
              "min-w-40 sm:w-fit",
              "min-h-fit",
              "px-3 py-2",
              "overflow-x-auto scrollbar-none",
              "bg-background-secondary",
              "sm:shadow sm:border-r",
              "border-b sm:border-b-0",
            )}
          >
            {pages.map((page) => (
              <SidebarLink
                href={`/t/${selectedTeam?.slug}/settings/${
                  page === "general" ? "" : page
                }`}
                isActive={page === selectedPage}
                key={page}
              >
                {startCase(page)}
              </SidebarLink>
            ))}
            <SidebarLink
              isActive={selectedPage === "audit-log"}
              href={`/t/${selectedTeam?.slug}/settings/audit-log`}
              disabled={!auditLogsEnabled}
              proBadge={!auditLogsEnabled}
            >
              Audit Log
            </SidebarLink>
          </aside>
          <div className="w-full overflow-y-auto scrollbar">
            <div className="flex h-full max-w-[65rem] flex-col gap-6 p-6">
              {selectedTeam ? (
                <Component team={selectedTeam} key={selectedTeam.id} />
              ) : (
                <Loading className="h-[50rem]" fullHeight={false} />
              )}
            </div>
          </div>
        </div>
      </PageContent>
    </>
  );
}
