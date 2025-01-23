import { Infer } from "convex/values";
import { ReactNode } from "react";
import { Doc, Id } from "system-udfs/convex/_generated/dataModel";
import {
  DatadogSiteLocation,
  ExportIntegrationType,
  IntegrationConfig,
  IntegrationType,
} from "system-udfs/convex/_system/frontend/common";
import {
  axiomConfig,
  datadogConfig,
  sentryConfig,
  webhookConfig,
} from "system-udfs/convex/schema";
import Link from "next/link";
import classNames from "classnames";
import { WebhookIcon } from "../elements/icons";
import { DatadogLogo } from "./logos/DatadogLogo";
import { AxiomLogo } from "./logos/AxiomLogo";
import { SentryLogo } from "./logos/SentryLogo";
import { AirbyteLogo } from "./logos/AirbyteLogo";
import { FivetranLogo } from "./logos/FivetranLogo";

export type SinkStatus = Doc<"_log_sinks">["status"];

export const LOG_INTEGRATIONS = ["axiom", "datadog", "webhook"] as const;
export const EXC_INTEGRATIONS = ["sentry"] as const;
export const EXPORT_INTEGRATIONS: ExportIntegrationType[] = [
  "fivetran",
  "airbyte",
];

export type LogIntegrationConfig =
  | Infer<typeof axiomConfig>
  | Infer<typeof datadogConfig>
  | Infer<typeof webhookConfig>;

export type LogIntegration =
  | {
      kind: "datadog";
      existing: {
        _id: Id<"_log_sinks">;
        _creationTime: number;
        status: SinkStatus;
        config: Infer<typeof datadogConfig>;
      } | null;
    }
  | {
      kind: "axiom";
      existing: {
        _id: Id<"_log_sinks">;
        _creationTime: number;
        status: SinkStatus;
        config: Infer<typeof axiomConfig>;
      } | null;
    }
  | {
      kind: "webhook";
      existing: {
        _id: Id<"_log_sinks">;
        _creationTime: number;
        status: SinkStatus;
        config: Infer<typeof webhookConfig>;
      } | null;
    };

export type ExceptionReportingIntegration = {
  kind: "sentry";
  existing: {
    _id: Id<"_log_sinks">;
    _creationTime: number;
    status: SinkStatus;
    config: Infer<typeof sentryConfig>;
  } | null;
};

export type ExceptionReportingIntegrationConfig = Infer<typeof sentryConfig>;

export function integrationToLogo(
  kind: IntegrationType,
  small?: boolean,
): {
  logo: ReactNode;
} {
  const sizeClass = small ? "size-5" : "size-10";
  switch (kind) {
    case "datadog":
      return {
        logo: (
          <DatadogLogo className={classNames("rounded border", sizeClass)} />
        ),
      };
    case "webhook":
      return {
        logo: (
          <div
            className={classNames(
              "flex items-center justify-center rounded border",
              sizeClass,
            )}
          >
            <WebhookIcon className={small ? "size-4" : "size-7"} />
          </div>
        ),
      };
    case "axiom":
      return {
        logo: <AxiomLogo className={classNames("rounded border", sizeClass)} />,
      };
    case "sentry":
      return {
        logo: (
          <SentryLogo className={classNames("rounded border", sizeClass)} />
        ),
      };
    case "airbyte":
      return {
        logo: (
          <AirbyteLogo className={classNames("rounded border", sizeClass)} />
        ),
      };
    case "fivetran":
      return {
        logo: (
          <FivetranLogo
            className={classNames(
              "rounded border bg-white dark:bg-black",
              sizeClass,
            )}
          />
        ),
      };
    default: {
      const _: never = kind;
      throw new Error(`Unrecognized integration type ${kind}`);
    }
  }
}

export function integrationUsingLegacyFormat(
  config: LogIntegrationConfig | ExceptionReportingIntegrationConfig | null,
) {
  if (config === null) {
    return false;
  }
  switch (config.type) {
    case "axiom":
      return config.version !== "2";
    case "datadog":
      return config.version !== "2";
    case "webhook":
      return false;
    case "sentry":
      return false;
    default: {
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const _typeCheck: never = config;
      return false;
    }
  }
}

export const LOG_STREAMS_DESCRIPTION = (
  <div>
    <p>Configure and monitor logging integrations within this deployment. </p>
    <p>
      Logs will be routed to your configured drains as functions are called and
      events occur in your deployment.
    </p>
    <p>
      This gives you full flexibility to query, store, and process logs as
      needed.
    </p>
  </div>
);

export const EXCEPTION_REPORTING_DESCRIPTION = (
  <div>
    <p>
      Configure and monitor exception reporting integrations within this
      deployment.
    </p>
    <p>
      Exceptions thrown from your Convex functions will be reported to your
      configured integrations.
    </p>
  </div>
);

export const STREAMING_EXPORT_DESCRIPTION = (
  <div>
    <p>Set up streaming export with third party connector platforms.</p>{" "}
    <p>
      Fivetran and Airbyte are data integration platforms that allow you to
      export your Convex data to other databases and data warehouses like
      Snowflake, Databricks, BigTable, Tableau, and many others.
    </p>
    <Link
      passHref
      href="https://docs.convex.dev/database/import-export/streaming"
      className="text-content-link dark:underline"
      target="_blank"
    >
      Learn more
    </Link>
    .
  </div>
);

export type IntegrationUnavailableReason =
  | "MissingEntitlement"
  | "CannotManageProd"
  | "LocalDeployment";

export const UNAVAILABLE_TOOLTIP_TEXT = {
  MissingEntitlement: "This integration requires a paid plan.",
  CannotManageProd:
    "You cannot manage integrations in a production deployment.",
  LocalDeployment: "You cannot manage integrations in a local deployment.",
};

export function configToUrl(config: IntegrationConfig): string {
  const kind = config.type;
  switch (kind) {
    case "sentry":
      return `https://sentry.io`;
    case "datadog":
      return datadogSiteLocationToUrl(config.siteLocation);
    case "axiom":
      return `https://app.axiom.co`;
    case "webhook":
      return config.url;
    default:
      // eslint-disable-next-line no-case-declarations
      const _: never = kind;
      throw new Error(`Unrecognized integration type ${kind}`);
  }
}

function datadogSiteLocationToUrl(siteLocation: DatadogSiteLocation): string {
  switch (siteLocation) {
    case "US1":
      return "https://datadoghq.com";
    case "US3":
      return "https://us3.datadoghq.com";
    case "US5":
      return "https://us5.datadoghq.com";
    case "EU":
      return "https://datadoghq.eu";
    case "US1_FED":
      return "https://ddog-gov.com";
    case "AP1":
      return "https://ap1.datadoghq.com";
    default: {
      const _: never = siteLocation;
      throw new Error(`Unrecognized site location ${siteLocation}`);
    }
  }
}

export const integrationName = (kind: IntegrationType) =>
  kind.charAt(0).toUpperCase() + kind.slice(1);
