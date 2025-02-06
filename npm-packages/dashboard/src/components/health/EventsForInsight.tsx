import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { Disclosure } from "@headlessui/react";
import {
  CaretDownIcon,
  ChevronDownIcon,
  ChevronUpIcon,
  ExternalLinkIcon,
  InfoCircledIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import {
  InsightsSummaryData,
  OCCEventData,
  useBytesReadEvents,
  useDocumentsReadEvents,
  useOCCFailedEvents,
  useOCCRetriedEvents,
} from "api/insights";
import {
  functionIdentifierValue,
  Button,
  Loading,
  Tooltip,
  FunctionNameOption,
  ComponentId,
  useNents,
  documentHref,
  formatBytes,
  formatNumberCompact,
} from "dashboard-common";
import { rootComponentPath } from "hooks/usageMetrics";
import { cn } from "lib/cn";
import Link from "next/link";
import { useContext } from "react";

export function EventsForInsight({
  insight,
}: {
  insight: InsightsSummaryData;
}) {
  return (
    <div className="flex flex-col gap-2 overflow-y-hidden">
      <div className="flex items-end justify-between">
        <div className="flex items-center gap-1">
          <h5>Recent Events</h5>
          <Tooltip
            tip="Recent events matching the criteria of this Insight."
            side="right"
          >
            <QuestionMarkCircledIcon />
          </Tooltip>
        </div>
        <span className="text-xs text-content-secondary">
          Data may be behind by a couple hours.
        </span>
      </div>
      {(() => {
        switch (insight.kind) {
          case "occFailedPermanently":
            return <OCCFailedEvents insight={insight} />;
          case "occRetried":
            return <OCCRetriedEvents insight={insight} />;
          case "bytesReadAverageThreshold":
          case "bytesReadCountThreshold":
            return <BytesReadEvents insight={insight} />;
          case "docsReadAverageThreshold":
          case "docsReadCountThreshold":
            return <DocumentsReadEvents insight={insight} />;
          default: {
            const _exhaustiveCheck: never = insight;
            return null;
          }
        }
      })()}
    </div>
  );
}

const eventTimestampColumn = {
  header: (
    <div className="flex items-center gap-1">
      Timestamp{" "}
      <Tooltip tip="Events are sorted in reverse cronological order.">
        <CaretDownIcon />
      </Tooltip>
    </div>
  ),
  className: "w-40",
  key: "timestamp",
  Column: EventTimestamp,
};
const eventFunctionCallColumn = {
  header: "Function Call ID",
  className: "w-32",
  key: "executionId",
  tooltip: "The unique ID of the function call that encountered this error.",
  Column: EventExecutionId,
};
const eventRequestIdColumn = {
  header: "Request ID",
  className: "w-36",
  key: "requestId",
  tooltip:
    "The ID of the request that triggered this function call. Multiple function calls may have the same Request ID if they are part of the same request, or if a function was retried.",
  Column: EventRequestId,
};
const eventOccWriteSourceColumn = {
  header: "Conflicting Function",
  className: "w-60",
  key: "occWriteSource",
  tooltip: "The function that caused the write conflict.",
  Column: EventOccWriteSource,
};
const eventOccDocumentIdColumn = {
  header: "Conflicting Document ID",
  className: "w-60",
  key: "occDocumentId",
  tooltip: "The ID of the document that caused the write conflict.",
  Column: EventOccDocumentId,
};
const eventStatusColumn = {
  header: "Status",
  className: "w-16",
  key: "status",
  tooltip: "Whether the function call ultimately succeeded or failed.",
  Column: EventStatus,
};

function BytesReadEvents({
  insight,
}: {
  insight: InsightsSummaryData & {
    kind: "bytesReadAverageThreshold" | "bytesReadCountThreshold";
  };
}) {
  const events = useBytesReadEvents({
    functionId: insight.functionId,
    componentPath: insight.componentPath || rootComponentPath,
  });

  return (
    <EventsTable
      events={events}
      insight={insight}
      columns={[
        eventRequestIdColumn,
        eventFunctionCallColumn,
        eventTimestampColumn,
        eventStatusColumn,
        {
          header: "Bytes Read",
          className: "w-60",
          key: "bytesRead",
          tooltip: "The number of bytes read by the function during this call.",
          Column: BytesEventReadAmount,
        },
      ]}
    />
  );
}

function DocumentsReadEvents({
  insight,
}: {
  insight: InsightsSummaryData & {
    kind: "docsReadAverageThreshold" | "docsReadCountThreshold";
  };
}) {
  const events = useDocumentsReadEvents({
    functionId: insight.functionId,
    componentPath: insight.componentPath || rootComponentPath,
  });

  return (
    <EventsTable
      events={events}
      insight={insight}
      columns={[
        eventRequestIdColumn,
        eventFunctionCallColumn,
        eventTimestampColumn,
        eventStatusColumn,
        {
          header: "Documents Read",
          className: "w-60",
          key: "bytesRead",
          tooltip:
            "The number of documents read by the function during this call.",
          Column: DocumentsEventReadAmount,
        },
      ]}
    />
  );
}

function OCCFailedEvents({
  insight,
}: {
  insight: InsightsSummaryData & {
    kind: "occFailedPermanently";
  };
}) {
  const events = useOCCFailedEvents({
    functionId: insight.functionId,
    tableName: insight.occTableName,
    componentPath: insight.componentPath || rootComponentPath,
  });

  return (
    <EventsTable
      events={events}
      insight={insight}
      columns={[
        eventRequestIdColumn,
        eventFunctionCallColumn,
        eventTimestampColumn,
        eventOccDocumentIdColumn,
        eventOccWriteSourceColumn,
      ]}
    />
  );
}

function OCCRetriedEvents({
  insight,
}: {
  insight: InsightsSummaryData & {
    kind: "occRetried";
  };
}) {
  const events = useOCCRetriedEvents({
    functionId: insight.functionId,
    tableName: insight.occTableName,
    componentPath: insight.componentPath || rootComponentPath,
  });

  return (
    <EventsTable
      events={events}
      insight={insight}
      columns={[
        eventRequestIdColumn,
        eventFunctionCallColumn,
        eventTimestampColumn,
        eventOccDocumentIdColumn,
        eventOccWriteSourceColumn,
        {
          header: "Retry #",
          className: "flex w-16 items-center gap-1",
          key: "occRetryCount",
          tooltip:
            "The number of previous attempts before this one to call the function.",
          Column: EventOccRetryCount,
        },
      ]}
    />
  );
}

/**
 * EventsTable is a generic table for displaying events.
 * It takes in a list of events and a list of columns to display.
 * Each column has a header, a className, a key, a tooltip, and a Column component.
 * The Column component is a React component that takes in an event and returns a JSX element.
 * The EventsTable component maps over the events and columns to render the table.
 * The table is scrollable and has a sticky header.
 */
function EventsTable<T, I extends InsightsSummaryData>({
  events,
  insight,
  columns,
}: {
  events?: T[];
  insight: I;
  columns: {
    header: React.ReactNode;
    className: string;
    key: string;
    tooltip?: string;
    Column: React.FC<{
      event: T;
      insight: I;
      componentId: ComponentId | undefined;
    }>;
  }[];
}) {
  const { nents } = useNents();
  const nentId = nents?.find((nent) => nent.path === insight.componentPath)?.id;
  return (
    <div className="flex max-h-full w-full flex-col overflow-y-auto rounded border scrollbar">
      <div className="sticky top-0 z-20 flex min-w-fit gap-2 border-b bg-background-secondary px-2 pb-1 pt-2 text-xs text-content-secondary">
        {columns.map((col) => (
          <div
            key={col.key}
            className={cn("flex items-center gap-1", col.className)}
          >
            {col.header}
            {col.tooltip && (
              <Tooltip tip={col.tooltip}>
                <QuestionMarkCircledIcon />
              </Tooltip>
            )}
          </div>
        ))}
      </div>
      <div className="flex max-h-full grow flex-col">
        {events &&
          events.map((event, idx) => (
            <div
              key={idx}
              className="flex min-w-fit animate-fadeInFromLoading gap-2 border-b p-2 text-xs last:border-b-0"
            >
              {columns.map((col) => (
                <col.Column
                  key={col.key}
                  event={event}
                  insight={insight}
                  componentId={nentId}
                />
              ))}
            </div>
          ))}
        {!events &&
          Array.from(
            {
              length: Math.min(
                7,
                insight.kind === "occRetried" ||
                  insight.kind === "occFailedPermanently"
                  ? insight.occCalls
                  : insight.aboveThresholdCalls,
              ),
            },
            (_, i) => i,
          ).map((i) => (
            <div
              key={i}
              className="flex w-full gap-2 border-b p-2 last:border-b-0"
            >
              {columns.map((col) => (
                <Loading className="h-4" key={col.key}>
                  <div className={col.className} />
                </Loading>
              ))}
            </div>
          ))}
      </div>
    </div>
  );
}

function EventTimestamp({ event }: { event: { timestamp: string } }) {
  return (
    <div className="min-w-40">
      {new Date(`${event.timestamp} UTC`).toLocaleString()}
    </div>
  );
}

function EventExecutionId({ event }: { event: { executionId: string } }) {
  return <div className="w-32 truncate">{event.executionId}</div>;
}

function EventRequestId({
  event,
  insight: _insight,
  componentId: _componentId,
}: {
  event: { requestId: string };
  insight: InsightsSummaryData;
  componentId: ComponentId | undefined;
}) {
  return (
    <div className="w-36">
      {event.requestId || (
        <span className="text-content-secondary">Unknown</span>
      )}
    </div>
  );
}

function EventOccDocumentId({
  insight,
  event,
  componentId,
}: {
  insight: InsightsSummaryData & {
    kind: "occFailedPermanently" | "occRetried";
  };
  event: OCCEventData;
  componentId: ComponentId | undefined;
}) {
  const { deploymentsURI } = useContext(DeploymentInfoContext);
  return (
    <div className="flex w-60">
      <Link
        href={documentHref(
          deploymentsURI,
          insight.occTableName,
          event.occDocumentId,
          componentId || undefined,
        )}
        target="_blank"
        className="flex items-center gap-1 text-content-link hover:underline"
      >
        {event.occDocumentId}
        <ExternalLinkIcon className="size-3 shrink-0" />
      </Link>
    </div>
  );
}

function EventOccWriteSource({
  insight,
  event,
}: {
  insight: InsightsSummaryData & {
    kind: "occFailedPermanently" | "occRetried";
  };
  event: OCCEventData;
}) {
  return (
    <div className="w-60 truncate">
      {!event.occWriteSource && "Unknown"}
      {event.occWriteSource &&
        (insight.functionId === event.occWriteSource ? (
          <span className="flex items-center gap-1 text-content-secondary">
            Self{" "}
            <Tooltip tip="Two calls to the same function resulted in this write conflict.">
              <InfoCircledIcon />
            </Tooltip>
          </span>
        ) : (
          <FunctionNameOption
            label={functionIdentifierValue(event.occWriteSource)}
            oneLine
          />
        ))}
    </div>
  );
}

function EventOccRetryCount({ event }: { event: { occRetryCount: number } }) {
  return <div className="w-16">{event.occRetryCount}</div>;
}

function BytesEventReadAmount({
  event,
}: {
  event: {
    totalCount: number;
    events: { tableName: string; count: number }[];
  };
}) {
  return <EventReadAmount event={event} format={formatBytes} />;
}

function DocumentsEventReadAmount({
  event,
}: {
  event: {
    totalCount: number;
    events: { tableName: string; count: number }[];
  };
}) {
  return <EventReadAmount event={event} format={formatNumberCompact} />;
}

function EventReadAmount({
  event,
  format,
}: {
  event: {
    totalCount: number;
    events: { tableName: string; count: number }[];
  };
  format: (count: number) => string;
}) {
  return (
    <div className="w-60">
      <Disclosure>
        {({ open }) => (
          <>
            <div className="flex items-center gap-1">
              <span className="min-w-[4.25rem]">
                {format(event.totalCount)}
              </span>
              <Disclosure.Button
                as={Button}
                inline
                variant="neutral"
                size="xs"
                tipSide="right"
                tip="View breakdown"
                className="-my-1"
              >
                {open ? <ChevronUpIcon /> : <ChevronDownIcon />}
              </Disclosure.Button>
            </div>
            <Disclosure.Panel>
              <ul className="mt-2 flex animate-fadeInFromLoading flex-col gap-1">
                {event.events.map((e, idx) => (
                  <li
                    key={idx}
                    className="list-inside list-decimal text-xs text-content-secondary"
                  >
                    <span className="font-semibold">{format(e.count)}</span>{" "}
                    from <span className="font-semibold">{e.tableName}</span>
                  </li>
                ))}
              </ul>
            </Disclosure.Panel>
          </>
        )}
      </Disclosure>
    </div>
  );
}

function EventStatus({
  event,
  insight: _insight,
  componentId: _componentId,
}: {
  event: {
    timestamp: string;
    executionId: string;
    totalCount: number;
    events: { tableName: string; count: number }[];
    status: string;
  };
  insight: InsightsSummaryData & {
    kind:
      | "bytesReadAverageThreshold"
      | "bytesReadCountThreshold"
      | "docsReadAverageThreshold"
      | "docsReadCountThreshold";
  };
  componentId: ComponentId | undefined;
}) {
  return (
    <div className="w-16">
      {event.status === "success" ? (
        "Success"
      ) : event.status === "failure" ? (
        "Failure"
      ) : (
        <span className="text-content-secondary">Unknown</span>
      )}
    </div>
  );
}
