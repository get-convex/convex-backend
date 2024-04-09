import { Infer } from "convex/values";
import {
  axiomConfig,
  completedExport,
  datadogConfig,
  udfType,
  udfVisibility,
  webhookConfig,
} from "../../schema";
import { Doc } from "../../_generated/dataModel";

export type UdfType = Infer<typeof udfType>;
export type Visibility = Infer<typeof udfVisibility>;

export type UdfWrite = {
  path: string;
  source: string;
};

export type LogLevel = "LOG" | "DEBUG" | "INFO" | "WARN" | "ERROR";

export type StructuredLogLine = {
  messages: string[];
  level: LogLevel;
  timestamp: number;
  isTruncated: boolean;
};
export type LogLine = string | StructuredLogLine;

export type FunctionExecutionCompletion = {
  kind: "Completion";
  identifier: string;
  udfType: UdfType;
  arguments: string[];
  logLines: LogLine[];
  // Unix timestamp (in seconds)
  timestamp: number;

  // null, except for successful http udfs where the status code will be
  // present. Always null for function udfs.
  // For http udfs, status is a string, but always a numeric value (200, 500
  // etc).
  success: { status: string } | null;
  error: string | null;

  cachedResult: boolean;
  // UDF execution duration (in seconds)
  executionTime: number;

  requestId: string;
  executionId: string;
};

export type FunctionExecutionProgess = {
  kind: "Progress";
  identifier: string;
  udfType: UdfType;
  // Unix timestamp (in seconds)
  timestamp: number;
  logLines: LogLine[];
  requestId: string;
  executionId: string;
};

export type FunctionExecution =
  | FunctionExecutionCompletion
  | FunctionExecutionProgess;

export type ResolvedSourcePos = {
  path: string;
  start_lineno: bigint;
  start_col: bigint;
};

export type AnalyzedModuleFunction = {
  name: string;
  lineno?: number;
  udfType: UdfType;
  visibility: Visibility;
  argsValidator: string;
};

// To deprecate
export type Module = {
  functions: AnalyzedModuleFunction[];
  cronSpecs?: [string, CronSpec][];
  creationTime: number;
};

export type CronJob = Doc<"_cron_jobs">;
export type CronSpec = Doc<"_cron_jobs">["cronSpec"];
export type CronSchedule = Doc<"_cron_jobs">["cronSpec"]["cronSchedule"];
export type CronJobLog = Doc<"_cron_job_logs">;
export type CronJobWithLastRun = Doc<"_cron_jobs"> & {
  // only undefined while feature-flagged (but still might be null)
  lastRun: CronJobLog | null | undefined;
};

export type Modules = Map<string, Module>;

export type CompletedExport = Infer<typeof completedExport>;

export type Export = Doc<"_exports">;

export type EnvironmentVariable = Doc<"_environment_variables">;

export type ScheduledJob = Doc<"_scheduled_jobs">;

export type UdfConfig = Doc<"_udf_config">;

export type Integration = Doc<"_log_sinks">;

export type IntegrationConfig = Integration["config"];

// Export integrations aren't configured on convex's side, so they don't use _log_sinks table
export type ExportIntegrationType = "airbyte" | "fivetran";

export type IntegrationType = IntegrationConfig["type"] | ExportIntegrationType;

export type DatadogConfig = Infer<typeof datadogConfig>;

export type DatadogSiteLocation = DatadogConfig["siteLocation"];

export type WebhookConfig = Infer<typeof webhookConfig>;

export type AxiomConfig = Infer<typeof axiomConfig>;
