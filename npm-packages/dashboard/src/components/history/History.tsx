import { DateRangePicker, useDateFilters } from "elements/DateRangePicker";
import {
  Loading,
  toast,
  Sheet,
  DeploymentAuditLogEvent,
  DeploymentAuditLogFilters,
  usePaginatedDeploymentEvents,
} from "dashboard-common";
import { DeploymentEventContent } from "elements/DeploymentEventContent";
import { endOfDay, endOfToday, startOfDay } from "date-fns";
import { useCurrentTeam, useTeamMembers, useTeamEntitlements } from "api/teams";
import Link from "next/link";
import { useRouter } from "next/router";
import { useCallback, useEffect, useState } from "react";

const INITIAL_EVENTS_TO_LOAD = 10;
const PAGE_SIZE = 10;
const DISTANCE_FROM_BOTTOM_THRESHOLD_PX = 300;

export function History() {
  const router = useRouter();
  const team = useCurrentTeam();
  const { startDate, endDate, setDate } = useDateFilters();
  const entitlements = useTeamEntitlements(team?.id);
  const auditLogsEnabled = entitlements?.auditLogsEnabled;

  // Current day
  const maxEndDate = endOfToday();

  const minStartDate = startOfDay(new Date(2023, 0, 1));

  const filters: DeploymentAuditLogFilters = {
    minDate: startDate.getTime(),
    maxDate: endOfDay(endDate).getTime(),
  };

  if (auditLogsEnabled === undefined) {
    return <Loading />;
  }
  if (!auditLogsEnabled) {
    toast(
      "info",
      "Deployment history is only available on paid plans.",
      "upsell",
    );
    void router.push(`/t/${router.query.team}/settings/billing`);
    return null;
  }

  return (
    <div className="flex h-full w-full flex-col gap-4 p-6 py-4">
      <h3>Deployment History</h3>
      <DateRangePicker
        minDate={minStartDate}
        maxDate={maxEndDate}
        date={{ from: startDate, to: endDate }}
        setDate={setDate}
      />
      <HistoryList filters={filters} />
    </div>
  );
}

function HistoryList({ filters }: { filters: DeploymentAuditLogFilters }) {
  const currentTeam = useCurrentTeam();
  const teamMembers = useTeamMembers(currentTeam?.id) ?? [];

  const [parentElement, setParentElement] = useState<HTMLDivElement>();
  const handleParent = useCallback((node: HTMLDivElement) => {
    setParentElement(node);
  }, []);

  const { results, loadMore, status } = usePaginatedDeploymentEvents(
    filters,
    teamMembers,
    INITIAL_EVENTS_TO_LOAD,
  );

  // Keep track of scroll position.
  useEffect(() => {
    function onScroll() {
      if (parentElement) {
        const distanceFromBottom =
          parentElement.scrollHeight -
          (parentElement.scrollTop + parentElement.clientHeight);
        if (
          distanceFromBottom < DISTANCE_FROM_BOTTOM_THRESHOLD_PX &&
          status === "CanLoadMore"
        ) {
          loadMore(PAGE_SIZE);
        }
      }
    }
    parentElement && onScroll();
    parentElement && parentElement.addEventListener("scroll", onScroll);

    return function cleanup() {
      parentElement?.removeEventListener("scroll", onScroll);
    };
  }, [parentElement, status, loadMore]);

  if (status === "LoadingFirstPage") {
    return <Loading />;
  }

  return (
    <Sheet
      ref={handleParent}
      padding={false}
      className="flex h-full min-w-[600px] max-w-[1200px] flex-col overflow-y-auto py-4 scrollbar"
    >
      {results.length === 0 && status !== "LoadingMore" ? (
        <EmptyHistory />
      ) : (
        results.map((deploymentEvent: DeploymentAuditLogEvent) => (
          <div
            className="border-b px-6 py-2 last:border-b-0"
            key={deploymentEvent._id}
          >
            <DeploymentEventContent
              event={deploymentEvent}
              key={deploymentEvent._id}
            />
          </div>
        ))
      )}
    </Sheet>
  );
}

function EmptyHistory() {
  return (
    <div className="flex h-full flex-1 flex-col items-center justify-center">
      <div className="mx-2 mt-10 flex flex-col items-center gap-2 text-content-secondary">
        No deployment history matching the selected date range, try extending
        the date range above.
        <div>
          <Link
            passHref
            href="https://docs.convex.dev/dashboard/deployments/history"
            className="text-content-link dark:underline"
            target="_blank"
          >
            Learn more
          </Link>{" "}
          about this page.
        </div>
      </div>
    </div>
  );
}
