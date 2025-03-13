import {
  ChevronUpIcon,
  ChevronDownIcon,
  TrashIcon,
  PlusIcon,
  UpdateIcon,
} from "@radix-ui/react-icons";
import { useContext, useMemo } from "react";
import { Infer } from "convex/values";
import { Disclosure } from "@headlessui/react";
import {
  authDiff,
  componentDiff,
  cronDiffType,
  schemaDiffType,
} from "system-udfs/convex/tableDefs/deploymentAuditLogTable";
import { SchemaJson, displaySchema } from "@common/lib/format";
import { DeploymentAuditLogEvent } from "@common/lib/useDeploymentAuditLog";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { Button } from "@common/elements/Button";
import { ReadonlyCodeDiff } from "@common/elements/ReadonlyCode";
import { NentNameOption } from "@common/elements/NentSwitcher";
import { NENT_APP_PLACEHOLDER } from "@common/lib/useNents";

function useSchemaCode(schema: null | string): string {
  return useMemo(() => {
    if (!schema) return "";
    const schema_json: SchemaJson = JSON.parse(schema);
    return displaySchema(schema_json) ?? "";
  }, [schema]);
}

export function DeploymentEventContent({
  event,
}: {
  event: DeploymentAuditLogEvent;
}) {
  const { TeamMemberLink } = useContext(DeploymentInfoContext);
  let body;
  switch (event.action) {
    case "build_indexes":
      // There are old audit log events for building indexes that
      // are redundant with the schema diff in push config events.
      return null;

    case "push_config":
      body = <PushContent event={event} />;
      break;

    case "push_config_with_components":
      body = (
        <>
          {event.metadata.component_diffs.map(
            ({ component_path, component_diff }) => {
              const auth =
                component_path === null ? event.metadata.auth_diff : undefined;
              return (
                <PushContentForComponents
                  key={component_path}
                  diff={component_diff}
                  componentName={component_path}
                  auth={auth}
                />
              );
            },
          )}
        </>
      );
      break;

    case "snapshot_import":
      body = <SnapshotImportContent event={event} />;
      break;

    case "create_environment_variable":
    case "delete_environment_variable":
    case "update_environment_variable":
    case "replace_environment_variable":
    case "update_canonical_url":
    case "delete_canonical_url":
    case "change_deployment_state":
    case "clear_tables":
    default:
      body = null;
  }

  return (
    <div className="flex flex-col gap-2 text-sm">
      <div className="flex items-center justify-between">
        <div className="flex h-6 flex-wrap items-center gap-1">
          <TeamMemberLink
            memberId={Number(event.member_id)}
            name={event.memberName}
          />
          <ActionText event={event} />
        </div>
        <TimestampDistance date={new Date(event._creationTime)} />
      </div>
      {body && <div className="ml-4 rounded-md border px-3 py-2.5">{body}</div>}
    </div>
  );
}

export function ActionText({ event }: { event: DeploymentAuditLogEvent }) {
  const { CloudImport } = useContext(DeploymentInfoContext);
  switch (event.action) {
    case "create_environment_variable":
      return (
        <>
          <span>created the environment variable </span>
          <span className="font-mono font-semibold">
            {event.metadata.variable_name}
          </span>
        </>
      );
    case "delete_environment_variable":
      return (
        <>
          <span>deleted the environment variable </span>
          <span className="font-mono font-semibold">
            {event.metadata.variable_name}
          </span>
        </>
      );
    case "update_environment_variable":
      return (
        <>
          <span>updated the environment variable </span>
          <span className="font-mono font-semibold">
            {event.metadata.variable_name}
          </span>
        </>
      );

    case "replace_environment_variable":
      return (
        <>
          <span>renamed the environment variable from</span>
          <span className="font-mono font-semibold">
            {event.metadata.previous_variable_name}
          </span>
          <span> to </span>
          <span className="font-mono font-semibold">
            {event.metadata.variable_name}
          </span>
        </>
      );

    case "update_canonical_url":
      return (
        <>
          <span>updated the canonical URL for </span>
          <span className="font-mono font-semibold">
            {event.metadata.request_destination}
          </span>
          <span> to </span>
          <span className="font-mono font-semibold">{event.metadata.url}</span>
        </>
      );

    case "delete_canonical_url":
      return (
        <>
          <span>deleted the canonical URL for </span>
          <span className="font-mono font-semibold">
            {event.metadata.request_destination}
          </span>
        </>
      );

    case "build_indexes":
      return <span>updated indexes</span>;

    case "push_config":
      return <span>deployed functions</span>;

    case "push_config_with_components":
      return <span>deployed functions</span>;

    case "change_deployment_state":
      switch (event.metadata.new_state) {
        case "paused":
          return <span>paused the deployment</span>;
        case "running":
          return <span>resumed the deployment</span>;
        case "disabled":
          return <span>disabled the deployment</span>;
        default:
          // eslint-disable-next-line @typescript-eslint/no-unused-vars, no-case-declarations
          const _: never = event.metadata.new_state;
          return null;
      }

    case "clear_tables":
      return <span>cleared tables</span>;

    case "snapshot_import": {
      if (event.metadata.requestor.type === "cloudRestore") {
        return (
          <CloudImport
            sourceCloudBackupId={Number(
              event.metadata.requestor.sourceCloudBackupId,
            )}
          />
        );
      }
      let format = "";
      switch (event.metadata.import_format.format) {
        case "csv":
          format = "CSV";
          break;
        case "jsonl":
          format = "JSONL";
          break;
        case "json_array":
          format = "JSON";
          break;
        case "zip":
          format = "ZIP";
          break;
        default:
          // eslint-disable-next-line @typescript-eslint/no-unused-vars, no-case-declarations
          const _: never = event.metadata.import_format;
          return null;
      }
      return <span>imported a snapshot from a {format} file</span>;
    }

    default:
      // eslint-disable-next-line @typescript-eslint/no-unused-vars, no-case-declarations
      const _: never = event;
      return null;
  }
}

function Variable({ variableName }: { variableName: string }) {
  return <div className="font-mono font-semibold">{variableName}</div>;
}

function CronElement({ cronDiff }: { cronDiff: Infer<typeof cronDiffType> }) {
  const hasCronDiff =
    cronDiff &&
    (cronDiff.added.length > 0 ||
      cronDiff.updated.length > 0 ||
      cronDiff.deleted.length > 0);
  if (!hasCronDiff) {
    return null;
  }
  const cronElement = (
    <div>
      {cronDiff.added.map((name) => (
        <div key={name} className="flex items-start gap-1.5">
          <Added />
          <span>cron job </span>
          <span className="font-mono font-semibold">{name}</span>
        </div>
      ))}
      {cronDiff.updated.map((name) => (
        <div key={name} className="flex items-start gap-1.5">
          <Updated />
          <span>cron job </span>
          <span className="font-mono font-semibold">{name}</span>
        </div>
      ))}
      {cronDiff.deleted.map((name) => (
        <div key={name} className="flex items-start gap-1.5">
          <Deleted />
          <span>cron job </span>
          <span className="font-mono font-semibold">{name}</span>
        </div>
      ))}
    </div>
  );
  return (
    <Disclosure>
      {({ open }) => (
        <>
          <div className="flex items-center gap-1.5">
            <span>Updated cron jobs</span>
            <Disclosure.Button as={Button} inline variant="neutral">
              {open ? <ChevronUpIcon /> : <ChevronDownIcon />}
            </Disclosure.Button>
          </div>

          <Disclosure.Panel className="ps-4">{cronElement}</Disclosure.Panel>
        </>
      )}
    </Disclosure>
  );
}

type ServerVersion = { previous_version: string; next_version: string } | null;
function ServerVersionChange({
  serverVersion,
}: {
  serverVersion: ServerVersion;
}) {
  return serverVersion ? (
    <div className="flex items-center gap-1 text-sm">
      <span>Set the convex package version to</span>
      <Variable variableName={serverVersion.next_version} />
    </div>
  ) : null;
}

function SchemaElement(diff: Infer<typeof schemaDiffType>) {
  const previousSchemaCode = useSchemaCode(diff?.previous_schema ?? null);
  const nextSchemaCode = useSchemaCode(diff?.next_schema ?? null);
  if (previousSchemaCode === nextSchemaCode) {
    return null;
  }
  return (
    <Disclosure>
      {({ open }) => (
        <>
          <div className="flex items-center gap-1.5">
            <span>Updated the schema</span>
            <Disclosure.Button as={Button} inline variant="neutral">
              {open ? <ChevronUpIcon /> : <ChevronDownIcon />}
            </Disclosure.Button>
          </div>

          <Disclosure.Panel className="ps-4">
            <ReadonlyCodeDiff
              language="javascript"
              path="schema.js"
              height={{ type: "content", maxHeightRem: 30 }}
              originalCode={previousSchemaCode}
              modifiedCode={nextSchemaCode}
            />
          </Disclosure.Panel>
        </>
      )}
    </Disclosure>
  );
}

function AuthElement({ diff }: { diff: Infer<typeof authDiff> }) {
  const hasAuthDiff = diff.added.length > 0 || diff.removed.length > 0;
  if (!hasAuthDiff) {
    return null;
  }
  const authElement = (
    <>
      {diff.added.map((newAuth, idx) => (
        <div className="flex items-start gap-1.5" key={idx}>
          <Added />
          <span className="font-mono font-semibold">{newAuth}</span>
        </div>
      ))}
      {diff.removed.map((removedAuth, idx) => (
        <div className="flex items-start gap-1.5" key={idx}>
          <Deleted />
          <span className="font-mono font-semibold">{removedAuth}</span>
        </div>
      ))}
    </>
  );
  return (
    <Disclosure>
      {({ open }) => (
        <>
          <div className="flex items-center gap-1.5">
            <span>Updated auth providers</span>
            <Disclosure.Button as={Button} inline variant="neutral">
              {open ? <ChevronUpIcon /> : <ChevronDownIcon />}
            </Disclosure.Button>
          </div>

          <Disclosure.Panel className="ps-4">{authElement}</Disclosure.Panel>
        </>
      )}
    </Disclosure>
  );
}

function PushContent({
  event,
}: {
  event: DeploymentAuditLogEvent & { action: "push_config" };
}) {
  const { auth, crons, server_version, schema } = event.metadata;
  const authElement = AuthElement({ diff: auth });

  const cronElement = <CronElement cronDiff={crons} />;

  const serverVersionChange = ServerVersionChange({
    serverVersion: server_version,
  });
  const schemaElement = SchemaElement(schema);
  return (
    <div className="flex flex-col gap-3 text-sm">
      <div className="flex items-center gap-1 text-sm">
        <UpdateIcon className="text-content-primary" />
        deployed functions
      </div>
      {serverVersionChange}
      {authElement}
      {cronElement}
      {schemaElement}
    </div>
  );
}

function PushContentForComponents({
  diff,
  auth,
  componentName,
}: {
  diff: Infer<typeof componentDiff>;
  auth?: Infer<typeof authDiff>;
  componentName: string | null;
}) {
  const { cronDiff, schemaDiff, udfConfigDiff, diffType } = diff;
  const cronElement = CronElement({ cronDiff });

  const serverVersionChange = ServerVersionChange({
    serverVersion: udfConfigDiff,
  });
  const schemaElement = SchemaElement(schemaDiff);
  let pastTenseDiff;
  switch (diffType.type) {
    case "create":
      pastTenseDiff = "Created";
      break;
    case "modify":
      pastTenseDiff = "Modified";
      break;
    case "unmount":
      pastTenseDiff = "Unmounted";
      break;
    case "remount":
      pastTenseDiff = "Remounted";
      break;
    default:
      // eslint-disable-next-line @typescript-eslint/no-unused-vars, no-case-declarations
      const _: never = diffType.type;
  }
  const authElement = auth ? AuthElement({ diff: auth }) : null;
  return (
    <div dir="ltr">
      <NentNameOption label={componentName ?? NENT_APP_PLACEHOLDER} inButton />
      <div className="my-2 ms-8">
        <div className="flex flex-col gap-3 text-sm">
          <ul className="list-disc">
            {componentName ? (
              <li>{pastTenseDiff} component</li>
            ) : (
              <li>Updated functions</li>
            )}
            {authElement && <li>{authElement}</li>}
            {serverVersionChange && <li>{serverVersionChange}</li>}
            {cronElement && <li>{cronElement}</li>}
            {schemaElement && <li>{schemaElement}</li>}
          </ul>
        </div>
      </div>
    </div>
  );
}

function Added() {
  return (
    <div className="flex items-center gap-1 text-sm">
      <PlusIcon className="h-3 w-3 text-content-primary" />
      added
    </div>
  );
}

function Updated() {
  return (
    <div className="flex items-center gap-1 text-sm">
      <UpdateIcon className="text-content-primary" />
      updated
    </div>
  );
}

function Deleted() {
  return (
    <div className="flex items-center gap-1 text-sm">
      <TrashIcon className="h-3 w-3 text-content-primary" />
      deleted
    </div>
  );
}

type TableNamesInput = { table_names: string[]; component: string | null }[];
type TableNameObject = { table_name: string; component: string | null };

function transformTableNames(input: TableNamesInput): TableNameObject[] {
  // Handle array of { table_names, component } objects
  return input.flatMap((item) =>
    item.table_names.map((table_name) => ({
      table_name,
      component: item.component,
    })),
  );
}

function SnapshotImportContent({
  event,
}: {
  event: DeploymentAuditLogEvent & { action: "snapshot_import" };
}) {
  const table_names = transformTableNames(event.metadata.table_names);
  const table_names_deleted = transformTableNames(
    event.metadata.table_names_deleted,
  );
  const omittedTables = Number(event.metadata.table_count) - table_names.length;
  const omittedTablesDeleted =
    Number(event.metadata.table_count_deleted) - table_names_deleted.length;
  return (
    <div className="flex flex-col gap-3 text-sm">
      {table_names.map(({ table_name, component }) => (
        <SnapshotImportIntoTable
          key={`${table_name} ${component}`}
          table={table_name}
          component={component}
          import_mode={event.metadata.import_mode}
        />
      ))}
      {omittedTables > 0 ? <span>and {omittedTables} more</span> : null}
      {table_names_deleted.map(({ table_name, component }) => (
        <SnapshotImportIntoTable
          key={`${table_name} ${component}`}
          table={table_name}
          component={component}
          import_mode={event.metadata.import_mode}
          deleted
        />
      ))}
      {omittedTablesDeleted > 0 ? (
        <span>and {omittedTablesDeleted} more</span>
      ) : null}
    </div>
  );
}

function SnapshotImportIntoTable({
  table,
  component,
  import_mode,
  deleted,
}: {
  table: string;
  component: string | null;
  import_mode: "RequireEmpty" | "Append" | "Replace" | "ReplaceAll";
  deleted?: boolean;
}) {
  let icon = null;
  let action = "";
  switch (import_mode) {
    case "RequireEmpty":
      icon = <PlusIcon className="h-3 w-3 text-content-primary" />;
      action = "created";
      break;
    case "Append":
      icon = <PlusIcon className="h-3 w-3 text-content-primary" />;
      action = "appended to";
      break;
    case "Replace":
    case "ReplaceAll":
      icon = <UpdateIcon className="text-content-primary" />;
      action = "replaced";
      break;
    default:
      console.error(`Unexpected import_mode ${import_mode}`);
      // eslint-disable-next-line @typescript-eslint/no-unused-vars, no-case-declarations
      const _: never = import_mode;
      return null;
  }

  if (deleted) {
    action = "deleted";
  }

  return (
    <div className="flex items-center gap-1 text-sm">
      {icon}
      {action}
      <Variable variableName={table} />
      {component !== null && (
        <>
          {" in component "}
          <Variable variableName={component} />
        </>
      )}
    </div>
  );
}
