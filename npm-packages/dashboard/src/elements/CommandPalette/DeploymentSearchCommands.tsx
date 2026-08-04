import { Command } from "cmdk";
import React, { useContext, useMemo } from "react";
import { ConvexProvider, useQuery } from "convex/react";
import type { Value } from "convex/values";
import { useRouter } from "next/router";
import {
  CodeIcon,
  DownloadIcon,
  ExternalLinkIcon,
  FileIcon,
  StopwatchIcon,
  TableIcon,
} from "@radix-ui/react-icons";
import udfs from "@common/udfs";
import {
  DeploymentInfoContext,
  useMaybeConnectedDeployment,
} from "@common/lib/deploymentContext";
import {
  documentHref,
  getReferencedTableName,
  getVisibleTableName,
} from "@common/lib/utils";
import { formatBytes } from "@common/lib/format";
import { stringifyValue } from "@common/lib/stringifyValue";
import { ReadonlyCode } from "@common/elements/ReadonlyCode";
import { DetailPanel } from "@common/elements/DetailPanel";
import { Button } from "@ui/Button";
import type { FileMetadata } from "system-udfs/convex/_system/frontend/fileStorageV2";
import {
  processAnalyzedModuleFunction,
  type ModuleFunction,
} from "@common/lib/functionHelpers";
import type { ComponentId } from "@common/lib/useNents";
import type { UdfType } from "system-udfs/convex/_system/frontend/common";
import { matchesSearch, NavigationDestination } from "./navigation";
import { REMOTE_VALUE_PREFIX } from "./navigation";
import { HighlightedText } from "./items";
import { PaletteCopyAction, useCopyAction } from "./copy";

const MAX_RESULTS = 20;

// Data-plane search within the current deployment: tables, functions,
// components, and documents (by ID).
export function DeploymentSearchCommands({
  search,
  onNavigate,
  onOpenDetail,
}: {
  search: string;
  onNavigate: (to: NavigationDestination) => void;
  // Opens the detail side panel for a document / file / scheduled-job match.
  onOpenDetail: (detail: SearchResultDetailItem) => void;
}) {
  const connected = useMaybeConnectedDeployment();
  if (!connected?.deployment) {
    return null;
  }
  return (
    <ConvexProvider client={connected.deployment.client}>
      <DeploymentSearchInner
        search={search}
        onNavigate={onNavigate}
        onOpenDetail={onOpenDetail}
      />
    </ConvexProvider>
  );
}

function DeploymentSearchInner({
  search,
  onNavigate,
  onOpenDetail,
}: {
  search: string;
  onNavigate: (to: NavigationDestination) => void;
  onOpenDetail: (detail: SearchResultDetailItem) => void;
}) {
  const router = useRouter();
  const { deploymentsURI, useIsOperationAllowed, captureMessage } = useContext(
    DeploymentInfoContext,
  );
  const canViewData = useIsOperationAllowed("ViewData");
  const trimmed = search.trim();
  // Fetch lazily: these subscriptions only start once the user types.
  const enabled = canViewData && trimmed.length > 0;

  // Search is scoped to the component the user is currently viewing
  const currentComponent =
    typeof router.query.component === "string" ? router.query.component : null;

  const tableMapping = useQuery(
    udfs.getTableMapping.default,
    enabled ? { componentId: currentComponent } : "skip",
  );
  const rawModules = useQuery(
    udfs.modules.list,
    enabled ? { componentId: currentComponent } : "skip",
  );

  const referencedTableName = getReferencedTableName(tableMapping, trimmed);
  const isStorageDoc = referencedTableName === "_file_storage";
  const isScheduledDoc = referencedTableName === "_scheduled_jobs";
  // Storage docs go through getFile so the preview has the download URL (and
  // image thumbnail); getById would only return the raw metadata row.
  const storageFile = useQuery(
    udfs.fileStorageV2.getFile,
    enabled && isStorageDoc
      ? { storageId: trimmed, componentId: currentComponent ?? undefined }
      : "skip",
  );
  const document = useQuery(
    udfs.getById.default,
    enabled && referencedTableName && !isStorageDoc
      ? { id: trimmed, componentId: currentComponent }
      : "skip",
  );

  const functions: ModuleFunction[] = useMemo(() => {
    if (!rawModules) {
      return [];
    }
    const result: ModuleFunction[] = [];
    for (const [filePath, module] of rawModules) {
      for (const fn of module.functions) {
        result.push(
          processAnalyzedModuleFunction(
            fn,
            filePath,
            currentComponent as ComponentId,
            null,
          ),
        );
      }
    }
    return result;
  }, [rawModules, currentComponent]);

  if (!enabled) {
    return null;
  }

  const matchingTables = Object.values(tableMapping ?? {})
    .filter((name) => !name.startsWith("_"))
    .filter((name) => matchesSearch(trimmed, name))
    .slice(0, MAX_RESULTS);

  const matchingFunctions = functions
    .filter((fn) => matchesSearch(trimmed, fn.displayName))
    .slice(0, MAX_RESULTS);

  return (
    <>
      {isStorageDoc && storageFile && (
        <StoragePreview
          file={storageFile}
          componentId={currentComponent}
          deploymentsURI={deploymentsURI}
          onOpenDetail={onOpenDetail}
        />
      )}
      {isScheduledDoc && document && (
        <ScheduledFunctionPreview
          job={document as ScheduledFunctionDoc}
          componentId={currentComponent}
          deploymentsURI={deploymentsURI}
          onOpenDetail={onOpenDetail}
        />
      )}
      {referencedTableName && !isStorageDoc && !isScheduledDoc && document && (
        <DocumentPreview
          tableName={referencedTableName}
          id={trimmed}
          document={document}
          componentId={currentComponent}
          deploymentsURI={deploymentsURI}
          captureMessage={captureMessage}
          onOpenDetail={onOpenDetail}
        />
      )}
      {matchingTables.length > 0 && (
        <Command.Group heading="Tables">
          {matchingTables.map((name) => (
            <TableResultItem
              key={name}
              tableName={name}
              componentId={currentComponent}
              deploymentsURI={deploymentsURI}
              onNavigate={onNavigate}
            />
          ))}
        </Command.Group>
      )}
      {matchingFunctions.length > 0 && (
        <Command.Group heading="Functions">
          {matchingFunctions.map((fn) => (
            <FunctionResultItem
              key={`${fn.componentId ?? ""}:${fn.identifier}`}
              fn={fn}
              deploymentsURI={deploymentsURI}
              onNavigate={onNavigate}
            />
          ))}
        </Command.Group>
      )}
    </>
  );
}

// A document ID is unique within a component, so this is the only match: one
// selectable row previewing the document's contents; selecting it opens the
// document in the detail side panel.
function DocumentPreview({
  tableName,
  id,
  document,
  componentId,
  deploymentsURI,
  captureMessage,
  onOpenDetail,
}: {
  tableName: string;
  id: string;
  document: Value;
  componentId: string | null;
  deploymentsURI: string;
  captureMessage: React.ContextType<
    typeof DeploymentInfoContext
  >["captureMessage"];
  onOpenDetail: (detail: SearchResultDetailItem) => void;
}) {
  return (
    <Command.Group heading={`Document in ${getVisibleTableName(tableName)}`}>
      <ResultItem
        value={`${REMOTE_VALUE_PREFIX}document-open:${id}`}
        Icon={TableIcon}
        title={id}
        titleClassName="font-mono"
        preview={stringifyValue(document)}
        copy={{
          label: "document",
          getText: () => stringifyValue(document, true),
        }}
        onSelect={() =>
          onOpenDetail({
            type: "document",
            tableName,
            id,
            value: document,
            target: documentHref({
              deploymentsURI,
              tableName,
              id,
              componentId,
              captureMessage,
            }),
          })
        }
      />
    </Command.Group>
  );
}

// One accessible result row: a leading icon (or custom leading node), a title
// with a muted content preview beneath it, and the navigable action on the
// right. Selecting the row performs the action.
function ResultItem({
  value,
  Icon,
  leading,
  title,
  titleClassName,
  preview,
  action,
  copy,
  onSelect,
}: {
  value: string;
  Icon?: React.FC<{ className?: string }>;
  leading?: React.ReactNode;
  title: string;
  titleClassName?: string;
  preview: React.ReactNode;
  // Right-aligned hint. Omitted for rows that open a detail panel on select.
  action?: string;
  copy: PaletteCopyAction;
  onSelect: () => void;
}) {
  useCopyAction(value, copy);
  return (
    <Command.Item
      value={value}
      className="animate-fadeInFromLoading"
      onSelect={onSelect}
    >
      {leading ?? (Icon && <Icon className="text-content-secondary" />)}
      <span className="flex min-w-0 flex-col">
        <span className={`truncate ${titleClassName ?? ""}`}>{title}</span>
        <span className="truncate text-xs text-content-tertiary">
          {preview}
        </span>
      </span>
      {action && (
        <span className="ml-auto shrink-0 text-xs text-content-tertiary">
          {action}
        </span>
      )}
    </Command.Item>
  );
}

function FunctionResultItem({
  fn,
  deploymentsURI,
  onNavigate,
}: {
  fn: ModuleFunction;
  deploymentsURI: string;
  onNavigate: (to: NavigationDestination) => void;
}) {
  const value = `${REMOTE_VALUE_PREFIX}function:${fn.componentId ?? ""}:${fn.identifier}`;
  useCopyAction(value, {
    label: "function name",
    getText: () => fn.displayName,
  });
  return (
    <Command.Item
      value={value}
      className="animate-fadeInFromLoading"
      onSelect={() =>
        onNavigate({
          pathname: `${deploymentsURI}/functions`,
          query: {
            function: fn.displayName,
            // `component` (the component ID) is the param useNents and the rest
            // of the page key off of.
            ...(fn.componentId ? { component: fn.componentId } : {}),
          },
        })
      }
    >
      <CodeIcon className="text-content-secondary" />
      <span className="truncate">
        <HighlightedText text={fn.displayName} />
      </span>
      <span className="ml-auto shrink-0 text-xs text-content-tertiary">
        {udfTypeLabel(fn.udfType)}
      </span>
    </Command.Item>
  );
}

function TableResultItem({
  tableName,
  componentId,
  deploymentsURI,
  onNavigate,
}: {
  tableName: string;
  componentId: string | null;
  deploymentsURI: string;
  onNavigate: (to: NavigationDestination) => void;
}) {
  const value = `${REMOTE_VALUE_PREFIX}table:${componentId ?? ""}:${tableName}`;
  useCopyAction(value, { label: "table name", getText: () => tableName });
  return (
    <Command.Item
      value={value}
      className="animate-fadeInFromLoading"
      onSelect={() =>
        onNavigate({
          pathname: `${deploymentsURI}/data`,
          query: {
            table: tableName,
            ...(componentId ? { component: componentId } : {}),
          },
        })
      }
    >
      <TableIcon className="text-content-secondary" />
      <span className="truncate">
        <HighlightedText text={tableName} />
      </span>
      <span className="ml-auto shrink-0 text-xs text-content-tertiary">
        Table
      </span>
    </Command.Item>
  );
}

function udfTypeLabel(udfType: UdfType): string {
  return udfType === "HttpAction" ? "HTTP Action" : udfType;
}

// The public `_scheduled_functions` shape (`db.system.get`), which differs from
// the internal `_scheduled_jobs` row: `state.kind` rather than `state.type`.
type ScheduledFunctionDoc = {
  _id: string;
  _creationTime: number;
  name: string;
  args: Value[];
  scheduledTime: number;
  completedTime?: number;
  state:
    | { kind: "pending" }
    | { kind: "inProgress" }
    | { kind: "success" }
    | { kind: "failed"; error: string }
    | { kind: "canceled" };
};

// The `_storage` file: one row previewing the file's type and size; selecting
// it opens the file in the detail side panel.
function StoragePreview({
  file,
  componentId,
  deploymentsURI,
  onOpenDetail,
}: {
  file: FileMetadata;
  componentId: string | null;
  deploymentsURI: string;
  onOpenDetail: (detail: SearchResultDetailItem) => void;
}) {
  return (
    <Command.Group heading="File">
      <ResultItem
        value={`${REMOTE_VALUE_PREFIX}file-open:${file._id}`}
        Icon={FileIcon}
        title={file._id}
        titleClassName="font-mono"
        preview={`${file.contentType || "Unknown type"} · ${formatBytes(Number(file.size))}`}
        copy={{ label: "storage ID", getText: () => file._id }}
        onSelect={() =>
          onOpenDetail({
            type: "file",
            file,
            target: {
              pathname: `${deploymentsURI}/files`,
              query: {
                id: file._id,
                ...(componentId ? { component: componentId } : {}),
              },
            },
          })
        }
      />
    </Command.Group>
  );
}

const SCHEDULED_STATE_LABEL: Record<
  ScheduledFunctionDoc["state"]["kind"],
  { label: string; className: string }
> = {
  pending: { label: "Pending", className: "text-content-warning" },
  inProgress: { label: "Running", className: "text-content-primary" },
  success: { label: "Success", className: "text-content-success" },
  failed: { label: "Failed", className: "text-content-error" },
  canceled: { label: "Canceled", className: "text-content-tertiary" },
};

// A scheduled run: one row previewing its status and arguments; selecting it
// opens the job in the detail side panel.
function ScheduledFunctionPreview({
  job,
  componentId,
  deploymentsURI,
  onOpenDetail,
}: {
  job: ScheduledFunctionDoc;
  componentId: string | null;
  deploymentsURI: string;
  onOpenDetail: (detail: SearchResultDetailItem) => void;
}) {
  const status = SCHEDULED_STATE_LABEL[job.state.kind];
  return (
    <Command.Group heading="Scheduled function">
      <ResultItem
        value={`${REMOTE_VALUE_PREFIX}scheduled-open:${componentId ?? ""}:${job._id}`}
        Icon={StopwatchIcon}
        title={job.name}
        titleClassName="font-mono"
        preview={
          <>
            <span className={status.className}>{status.label}</span>
            {job.state.kind === "failed" && ` · ${job.state.error}`}
            {" · "}
            {stringifyValue(job.args)}
          </>
        }
        copy={{
          label: "document",
          getText: () => stringifyValue(job as unknown as Value, true),
        }}
        onSelect={() =>
          onOpenDetail({
            type: "scheduled",
            job,
            target: {
              pathname: `${deploymentsURI}/schedules/functions`,
              query: {
                function: job.name,
                ...(componentId ? { component: componentId } : {}),
              },
            },
          })
        }
      />
    </Command.Group>
  );
}

// A data-plane match the user can open in the detail side panel. The page
// navigation `target` is precomputed, so the panel — rendered outside the
// deployment providers — needs no deployment context to offer its button.
export type SearchResultDetailItem =
  | {
      type: "document";
      tableName: string;
      id: string;
      value: Value;
      target: NavigationDestination;
    }
  | { type: "file"; file: FileMetadata; target: NavigationDestination }
  | {
      type: "scheduled";
      job: ScheduledFunctionDoc;
      target: NavigationDestination;
    };

// Right-hand detail drawer for a data-plane search match: previews the item's
// contents (reusing the deployment's own viewers) and offers a button to open
// the corresponding page. Rendered by the top-level CommandPalette rather than
// the cmdk dialog, so only one focus-trapping dialog is ever active.
export function SearchResultDetail({
  detail,
  onNavigate,
  onClose,
}: {
  detail: SearchResultDetailItem;
  onNavigate: (to: NavigationDestination) => void;
  onClose: () => void;
}) {
  const { header, action, body } = detailContent(detail);
  return (
    <DetailPanel
      onClose={onClose}
      // Actions sit inline with the title rather than in their own row, so the
      // panel doesn't spend vertical space on a toolbar.
      header={
        <span className="flex flex-wrap items-center gap-x-3 gap-y-1">
          <span>{header}</span>
          <span className="flex gap-2">
            {detail.type === "file" && (
              <Button
                variant="neutral"
                size="sm"
                icon={<DownloadIcon />}
                href={detail.file.url}
                download
                target="_blank"
              >
                Download
              </Button>
            )}
            <Button
              variant="neutral"
              size="sm"
              icon={<ExternalLinkIcon />}
              onClick={() => onNavigate(detail.target)}
            >
              {action}
            </Button>
          </span>
        </span>
      }
      content={<div className="h-full">{body}</div>}
    />
  );
}

function detailContent(detail: SearchResultDetailItem): {
  header: string;
  action: string;
  body: React.ReactNode;
} {
  switch (detail.type) {
    case "document":
      return {
        header: `Document in ${getVisibleTableName(detail.tableName)}`,
        action: "View in Data",
        body: (
          <div className="h-full overflow-hidden rounded-sm border p-4">
            <ReadonlyCode
              path={`commandPalette-document-${detail.id}`}
              code={stringifyValue(detail.value, true)}
              disableLineNumbers
            />
          </div>
        ),
      };
    case "file":
      return {
        header: "File",
        action: "View in Files",
        body: <FileDetailBody file={detail.file} />,
      };
    case "scheduled":
      return {
        header: "Scheduled function",
        action: "View in Scheduled Jobs",
        body: <ScheduledDetailBody job={detail.job} />,
      };
    default: {
      detail satisfies never;
      return { header: "", action: "", body: null };
    }
  }
}

function DetailField({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex gap-3 text-sm">
      <span className="w-28 shrink-0 text-content-tertiary">{label}</span>
      <span className="min-w-0 wrap-break-word text-content-primary">
        {children}
      </span>
    </div>
  );
}

function FileDetailBody({ file }: { file: FileMetadata }) {
  return (
    <div className="flex flex-col gap-3">
      <DetailField label="Storage ID">
        <span className="font-mono">{file._id}</span>
      </DetailField>
      <DetailField label="Size">{formatBytes(Number(file.size))}</DetailField>
      <DetailField label="Content type">
        {file.contentType || "Unknown"}
      </DetailField>
      <DetailField label="Uploaded">
        {new Date(file._creationTime).toLocaleString()}
      </DetailField>
    </div>
  );
}

function ScheduledDetailBody({ job }: { job: ScheduledFunctionDoc }) {
  const status = SCHEDULED_STATE_LABEL[job.state.kind];
  return (
    <div className="flex flex-col gap-3">
      <DetailField label="Function">
        <span className="font-mono">{job.name}</span>
      </DetailField>
      <DetailField label="Status">
        <span className={status.className}>{status.label}</span>
        {job.state.kind === "failed" && (
          <span className="text-content-error"> · {job.state.error}</span>
        )}
      </DetailField>
      <DetailField label="Scheduled">
        {new Date(job.scheduledTime).toLocaleString()}
      </DetailField>
      {job.completedTime !== undefined && (
        <DetailField label="Completed">
          {new Date(job.completedTime).toLocaleString()}
        </DetailField>
      )}
      <DetailField label="Arguments">
        <span className="font-mono break-all">{stringifyValue(job.args)}</span>
      </DetailField>
    </div>
  );
}
