import chalk from "chalk";
import path from "path";
import { bundleSchema } from "../../bundler/index.js";
import { Context } from "../../bundler/context.js";
import {
  changeSpinner,
  logFailure,
  logFinishedStep,
  logError,
} from "../../bundler/log.js";
import {
  poll,
  logAndHandleFetchError,
  deploymentFetch,
  deprecationCheckWarning,
} from "./utils/utils.js";
import { deploymentDashboardUrlPage } from "./dashboard.js";

type IndexMetadata = {
  table: string;
  name: string;
  fields:
    | string[]
    | {
        searchField: string;
        filterFields: string[];
      };
  backfill: {
    state: "in_progress" | "done";
  };
  staged: boolean;
};

type SchemaState =
  | { state: "pending" }
  | { state: "validated" }
  | { state: "active" }
  | { state: "overwritten" }
  | { state: "failed"; error: string; tableName?: string };

type SchemaStateResponse = {
  indexes: IndexMetadata[];
  schemaState: SchemaState;
};
type PrepareSchemaResponse = {
  added: IndexMetadata[];
  dropped: IndexMetadata[];
  enabled: IndexMetadata[];
  disabled: IndexMetadata[];
  schemaId: string;
};

export async function pushSchema(
  ctx: Context,
  origin: string,
  adminKey: string,
  schemaDir: string,
  dryRun: boolean,
  deploymentName?: string | null,
): Promise<{ schemaId?: string; schemaState?: SchemaState }> {
  if (
    !ctx.fs.exists(path.resolve(schemaDir, "schema.ts")) &&
    !ctx.fs.exists(path.resolve(schemaDir, "schema.js"))
  ) {
    // Don't do anything.
    return {};
  }
  const bundles = await bundleSchema(ctx, schemaDir, []);

  changeSpinner("Checking for index or schema changes...");

  let data: PrepareSchemaResponse;
  const fetch = deploymentFetch(ctx, {
    deploymentUrl: origin,
    adminKey,
  });
  try {
    const res = await fetch("/api/prepare_schema", {
      method: "POST",
      body: JSON.stringify({
        bundle: bundles[0],
        adminKey,
        dryRun,
      }),
    });
    deprecationCheckWarning(ctx, res);
    data = await res.json();
  } catch (err: unknown) {
    logFailure(`Error: Unable to run schema validation on ${origin}`);
    return await logAndHandleFetchError(ctx, err);
  }

  logIndexChanges(ctx, data, dryRun);
  const schemaId = data.schemaId;
  const schemaState = await waitForReadySchema(
    ctx,
    origin,
    adminKey,
    schemaId,
    deploymentName,
  );
  return { schemaId, schemaState };
}

/// Wait for indexes to build and schema to be validated.
async function waitForReadySchema(
  ctx: Context,
  origin: string,
  adminKey: string,
  schemaId: string,
  deploymentName?: string | null,
): Promise<SchemaState> {
  const path = `api/schema_state/${schemaId}`;
  const depFetch = deploymentFetch(ctx, {
    deploymentUrl: origin,
    adminKey,
  });
  const fetch = async () => {
    try {
      const resp = await depFetch(path, { method: "GET" });
      const data: SchemaStateResponse = await resp.json();
      return data;
    } catch (err: unknown) {
      logFailure(
        `Error: Unable to build indexes and run schema validation on ${origin}`,
      );
      return await logAndHandleFetchError(ctx, err);
    }
  };

  // Set the spinner to the default progress message before the first `fetch` call returns.
  const start = Date.now();

  setSchemaProgressSpinner(ctx, null, start, deploymentName);

  const data = await poll(fetch, (data: SchemaStateResponse) => {
    setSchemaProgressSpinner(ctx, data, start, deploymentName);
    return (
      data.indexes.every(
        (index) => index.backfill.state === "done" || index.staged,
      ) && data.schemaState.state !== "pending"
    );
  });

  switch (data.schemaState.state) {
    case "failed":
      // Schema validation failed. This could be either because the data
      // is bad or the schema is wrong. Classify this as a filesystem error
      // because adjusting `schema.ts` is the most normal next step.
      logFailure("Schema validation failed");
      logError(chalk.red(`${data.schemaState.error}`));
      return await ctx.crash({
        exitCode: 1,
        errorType: {
          "invalid filesystem or db data": data.schemaState.tableName
            ? {
                tableName: data.schemaState.tableName,
              }
            : null,
        },
        printedMessage: null, // TODO - move logging into here
      });

    case "overwritten":
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Schema was overwritten by another push.`,
      });
    case "validated":
      logFinishedStep("Schema validation complete.");
      break;
    case "active":
      break;
  }
  return data.schemaState;
}

function setSchemaProgressSpinner(
  ctx: Context,
  data: SchemaStateResponse | null,
  start: number,
  deploymentName?: string | null,
) {
  if (!data) {
    changeSpinner("Pushing code to your deployment...");
    return;
  }
  const indexesCompleted = data.indexes.filter(
    (index) => index.backfill.state === "done",
  ).length;
  const numIndexes = data.indexes.length;

  const indexesDone = indexesCompleted === numIndexes;
  const schemaDone = data.schemaState.state !== "pending";

  if (indexesDone && schemaDone) {
    return;
  }

  let msg = "Pushing your code to your Convex deployment...";
  if (!indexesDone && !schemaDone) {
    msg = `Backfilling indexes (${indexesCompleted}/${numIndexes} ready) and checking that documents match your schema...`;
  } else if (!indexesDone) {
    if (Date.now() - start > 10_000 && deploymentName) {
      for (const index of data.indexes) {
        if (index.backfill.state === "in_progress") {
          const dashboardUrl = deploymentDashboardUrlPage(
            deploymentName,
            `/data?table=${index.table}&showIndexes=true`,
          );
          msg = `Backfilling index ${index.name} (${indexesCompleted}/${numIndexes} ready), \
see progress on the dashboard here: ${dashboardUrl}`;
          break;
        }
      }
    } else {
      msg = `Backfilling indexes (${indexesCompleted}/${numIndexes} ready)...`;
    }
  } else {
    msg = "Checking that documents match your schema...";
  }
  changeSpinner(msg);
}

function logIndexChanges(
  ctx: Context,
  indexes: PrepareSchemaResponse,
  dryRun: boolean,
) {
  if (indexes.dropped.length > 0) {
    let indexDiff = "";
    for (const index of indexes.dropped) {
      indexDiff += `  [-] ${stringifyIndex(index)}\n`;
    }
    // strip last new line
    indexDiff = indexDiff.slice(0, -1);
    logFinishedStep(
      `${dryRun ? "Would delete" : "Deleted"} table indexes:\n${indexDiff}`,
    );
  }
  if (indexes.added.length > 0) {
    let indexDiff = "";
    for (const index of indexes.added) {
      indexDiff += `  [+] ${stringifyIndex(index)}\n`;
    }
    // strip last new line
    indexDiff = indexDiff.slice(0, -1);
    logFinishedStep(
      `${dryRun ? "Would add" : "Added"} table indexes:\n${indexDiff}`,
    );
  }
  if (indexes.enabled.length > 0) {
    let indexDiff = "";
    for (const index of indexes.enabled) {
      indexDiff += `  [*] ${stringifyIndex(index)}\n`;
    }
    // strip last new line
    indexDiff = indexDiff.slice(0, -1);
    const text = dryRun
      ? `These indexes would be enabled`
      : `These indexes are now enabled`;
    logFinishedStep(`${text}:\n${indexDiff}`);
  }
  if (indexes.disabled.length > 0) {
    let indexDiff = "";
    for (const index of indexes.disabled) {
      indexDiff += `  [*] ${stringifyIndex(index)}\n`;
    }
    // strip last new line
    indexDiff = indexDiff.slice(0, -1);
    const text = dryRun
      ? `These indexes would be staged`
      : `These indexes are now staged`;
    logFinishedStep(`${text}:\n${indexDiff}`);
  }
}

function stringifyIndex(index: IndexMetadata) {
  return `${index.table}.${index.name} ${JSON.stringify(index.fields)}`;
}
