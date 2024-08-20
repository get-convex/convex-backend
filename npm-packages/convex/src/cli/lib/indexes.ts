import chalk from "chalk";
import path from "path";
import { bundleSchema } from "../../bundler/index.js";
import {
  Context,
  changeSpinner,
  logFailure,
  logFinishedStep,
  logError,
} from "../../bundler/context.js";
import {
  poll,
  logAndHandleFetchError,
  deploymentFetch,
  deprecationCheckWarning,
} from "./utils.js";

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
  schemaId: string;
};

export async function pushSchema(
  ctx: Context,
  origin: string,
  adminKey: string,
  schemaDir: string,
  dryRun: boolean,
): Promise<{ schemaId?: string; schemaState?: SchemaState }> {
  if (!ctx.fs.exists(path.resolve(schemaDir, "schema.ts"))) {
    // Don't do anything.
    return {};
  }
  const bundles = await bundleSchema(ctx, schemaDir);

  changeSpinner(ctx, "Checking for index or schema changes...");

  let data: PrepareSchemaResponse;
  const fetch = deploymentFetch(origin, adminKey);
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
    logFailure(ctx, `Error: Unable to run schema validation on ${origin}`);
    return await logAndHandleFetchError(ctx, err);
  }

  const schemaId = data.schemaId;

  const schemaState = await waitForReadySchema(ctx, origin, adminKey, schemaId);
  logIndexChanges(ctx, data, dryRun);
  return { schemaId, schemaState };
}

/// Wait for indexes to build and schema to be validated.
async function waitForReadySchema(
  ctx: Context,
  origin: string,
  adminKey: string,
  schemaId: string,
): Promise<SchemaState> {
  const path = `api/schema_state/${schemaId}`;
  const depFetch = deploymentFetch(origin, adminKey);
  const fetch = async () => {
    try {
      const resp = await depFetch(path, { method: "GET" });
      const data: SchemaStateResponse = await resp.json();
      return data;
    } catch (err: unknown) {
      logFailure(
        ctx,
        `Error: Unable to build indexes and run schema validation on ${origin}`,
      );
      return await logAndHandleFetchError(ctx, err);
    }
  };

  // Set the spinner to the default progress message before the first `fetch` call returns.
  setSchemaProgressSpinner(ctx, null);

  const data = await poll(fetch, (data: SchemaStateResponse) => {
    setSchemaProgressSpinner(ctx, data);
    return (
      data.indexes.every((index) => index.backfill.state === "done") &&
      data.schemaState.state !== "pending"
    );
  });

  switch (data.schemaState.state) {
    case "failed":
      // Schema validation failed. This could be either because the data
      // is bad or the schema is wrong. Classify this as a filesystem error
      // because adjusting `schema.ts` is the most normal next step.
      logFailure(ctx, "Schema validation failed");
      logError(ctx, chalk.red(`${data.schemaState.error}`));
      return await ctx.crash({
        exitCode: 1,
        errorType: {
          "invalid filesystem or db data": data.schemaState.tableName ?? null,
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
      logFinishedStep(ctx, "Schema validation complete.");
      break;
    case "active":
      break;
  }
  return data.schemaState;
}

function setSchemaProgressSpinner(
  ctx: Context,
  data: SchemaStateResponse | null,
) {
  if (!data) {
    changeSpinner(
      ctx,
      "Backfilling indexes and checking that documents match your schema...",
    );
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

  let msg: string;
  if (!indexesDone && !schemaDone) {
    msg = `Backfilling indexes (${indexesCompleted}/${numIndexes} ready) and checking that documents match your schema...`;
  } else if (!indexesDone) {
    msg = `Backfilling indexes (${indexesCompleted}/${numIndexes} ready)...`;
  } else {
    msg = "Checking that documents match your schema...";
  }
  changeSpinner(ctx, msg);
}

function logIndexChanges(
  ctx: Context,
  indexes: {
    added: IndexMetadata[];
    dropped: IndexMetadata[];
  },
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
      ctx,
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
      ctx,
      `${dryRun ? "Would add" : "Added"} table indexes:\n${indexDiff}`,
    );
  }
}

function stringifyIndex(index: IndexMetadata) {
  return `${index.table}.${index.name} ${JSON.stringify(index.fields)}`;
}
