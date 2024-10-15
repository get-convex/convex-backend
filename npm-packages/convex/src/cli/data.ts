import { Option } from "@commander-js/extra-typings";
import chalk from "chalk";
import {
  Context,
  logError,
  logOutput,
  logWarning,
  oneoffContext,
} from "../bundler/context.js";
import { Base64 } from "../values/index.js";
import { Value } from "../values/value.js";
import {
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsProvisionProd,
} from "./lib/api.js";
import { runPaginatedQuery } from "./lib/run.js";
import { parsePositiveInteger } from "./lib/utils/utils.js";
import { Command } from "@commander-js/extra-typings";
import { actionDescription } from "./lib/command.js";

export const data = new Command("data")
  .summary("List tables and print data from your database")
  .description(
    "Inspect your Convex deployment's database.\n\n" +
      "  List tables: `npx convex data`\n" +
      "  List documents in a table: `npx convex data tableName`\n\n" +
      "By default, this inspects your dev deployment.",
  )
  .allowExcessArguments(false)
  .argument("[table]", "If specified, list documents in this table.")
  .addOption(
    new Option(
      "--limit <n>",
      "List only the `n` the most recently created documents.",
    )
      .default(100)
      .argParser(parsePositiveInteger),
  )
  .addOption(
    new Option(
      "--order <choice>",
      "Order the documents by their `_creationTime`.",
    )
      .choices(["asc", "desc"])
      .default("desc"),
  )
  .addOption(
    new Option(
      "--component <path>",
      "Path to the component in the component tree defined in convex.config.ts.\n" +
        "  By default, inspects data in the root component",
    ).hideHelp(),
  )
  .addDeploymentSelectionOptions(actionDescription("Inspect the database in"))
  .showHelpAfterError()
  .action(async (tableName, options) => {
    const ctx = oneoffContext();
    const deploymentSelection = deploymentSelectionFromOptions(options);

    const {
      adminKey,
      url: deploymentUrl,
      deploymentName,
    } = await fetchDeploymentCredentialsProvisionProd(ctx, deploymentSelection);

    if (tableName !== undefined) {
      await listDocuments(ctx, deploymentUrl, adminKey, tableName, {
        ...options,
        order: options.order as "asc" | "desc",
        componentPath: options.component ?? "",
      });
    } else {
      await listTables(
        ctx,
        deploymentUrl,
        adminKey,
        deploymentName,
        options.component ?? "",
      );
    }
  });

async function listTables(
  ctx: Context,
  deploymentUrl: string,
  adminKey: string,
  deploymentName: string | undefined,
  componentPath: string,
) {
  const tables = (await runPaginatedQuery(
    ctx,
    deploymentUrl,
    adminKey,
    "_system/cli/tables",
    componentPath,
    {},
  )) as { name: string }[];
  if (tables.length === 0) {
    logError(
      ctx,
      `There are no tables in the ${
        deploymentName ? `${chalk.bold(deploymentName)} deployment's ` : ""
      }database.`,
    );
    return;
  }
  const tableNames = tables.map((table) => table.name);
  tableNames.sort();
  logOutput(ctx, tableNames.join("\n"));
}

async function listDocuments(
  ctx: Context,
  deploymentUrl: string,
  adminKey: string,
  tableName: string,
  options: {
    limit: number;
    order: "asc" | "desc";
    componentPath: string;
  },
) {
  const data = (await runPaginatedQuery(
    ctx,
    deploymentUrl,
    adminKey,
    "_system/cli/tableData",
    options.componentPath,
    {
      table: tableName,
      order: options.order ?? "desc",
    },
    options.limit + 1,
  )) as Record<string, Value>[];

  if (data.length === 0) {
    logError(ctx, "There are no documents in this table.");
    return;
  }

  logDocumentsTable(
    ctx,
    data.slice(0, options.limit).map((document) => {
      const printed: Record<string, string> = {};
      for (const key in document) {
        printed[key] = stringify(document[key]);
      }
      return printed;
    }),
  );
  if (data.length > options.limit) {
    logWarning(
      ctx,
      chalk.yellow(
        `Showing the ${options.limit} ${
          options.order === "desc" ? "most recently" : "oldest"
        } created document${
          options.limit > 1 ? "s" : ""
        }. Use the --limit option to see more.`,
      ),
    );
  }
}

function logDocumentsTable(ctx: Context, rows: Record<string, string>[]) {
  const columnsToWidths: Record<string, number> = {};
  for (const row of rows) {
    for (const column in row) {
      const value = row[column];
      columnsToWidths[column] = Math.max(
        value.length,
        columnsToWidths[column] ?? 0,
      );
    }
  }
  const unsortedFields = Object.keys(columnsToWidths);
  unsortedFields.sort();
  const fields = Array.from(
    new Set(["_id", "_creationTime", ...unsortedFields]),
  );
  const columnWidths = fields.map((field) => columnsToWidths[field]);
  const lineLimit = process.stdout.isTTY ? process.stdout.columns : undefined;

  let didTruncate = false;

  function limitLine(line: string, limit: number | undefined) {
    if (limit === undefined) {
      return line;
    }
    const limitWithBufferForUnicode = limit - 10;
    if (line.length > limitWithBufferForUnicode) {
      didTruncate = true;
    }
    return line.slice(0, limitWithBufferForUnicode);
  }

  logOutput(
    ctx,
    limitLine(
      fields.map((field, i) => field.padEnd(columnWidths[i])).join(" | "),
      lineLimit,
    ),
  );
  logOutput(
    ctx,
    limitLine(
      columnWidths.map((width) => "-".repeat(width)).join("-|-"),
      lineLimit,
    ),
  );
  for (const row of rows) {
    logOutput(
      ctx,
      limitLine(
        fields
          .map((field, i) => (row[field] ?? "").padEnd(columnWidths[i]))
          .join(" | "),
        lineLimit,
      ),
    );
  }
  if (didTruncate) {
    logWarning(
      ctx,
      chalk.yellow(
        "Lines were truncated to fit the terminal width. Pipe the command to see " +
          "the full output, such as:\n  `npx convex data tableName | less -S`",
      ),
    );
  }
}

function stringify(value: Value): string {
  if (value === null) {
    return "null";
  }
  if (typeof value === "bigint") {
    return `${value.toString()}n`;
  }
  if (typeof value === "number") {
    return value.toString();
  }
  if (typeof value === "boolean") {
    return value.toString();
  }
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (value instanceof ArrayBuffer) {
    const base64Encoded = Base64.fromByteArray(new Uint8Array(value));
    return `Bytes("${base64Encoded}")`;
  }
  if (value instanceof Array) {
    return `[${value.map(stringify).join(", ")}]`;
  }
  const pairs = Object.entries(value)
    .map(([k, v]) => `"${k}": ${stringify(v!)}`)
    .join(", ");
  return `{ ${pairs} }`;
}
