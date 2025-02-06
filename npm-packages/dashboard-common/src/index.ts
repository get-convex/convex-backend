/**
 * This barrel file exports all code that is shared between the
 * self-hosted dashboard and the cloud dashboard.
 */

// General dashboard exports
export { toast, dismissToast, backoffWithJitter } from "@common/lib/utils";
export * from "@common/lib/fetching";
export * from "@common/lib/useGlobalLocalStorage";
export * from "@common/lib/useCopy";
export * from "@common/lib/useIsOverflowing";
export {
  formatBytes,
  formatNumber,
  formatNumberCompact,
  formatDate,
  msFormat,
  toNumericUTC,
} from "@common/lib/format";

// Deployment-related exports for Insights
export { useFunctionUrl } from "@common/lib/deploymentApi";
export {
  functionIdentifierFromValue,
  functionIdentifierValue,
} from "@common/lib/functions/generateFileTree";
export * from "@common/lib/useNents";
export { useDeploymentAuditLogs } from "@common/lib/useDeploymentAuditLog";
export {
  useModuleFunctions,
  itemIdentifier,
} from "@common/lib/functions/FunctionsProvider";
export { documentHref } from "@common/lib/utils";
export * from "@common/elements/FunctionNameOption";
export * from "@common/elements/HealthCard";
export * from "@common/features/health/components/DeploymentTimes";

// Re-used in the cloud dashboard for deployment pages
export {
  PROVISION_PROD_PAGE_NAME,
  DeploymentInfoContext,
  ConnectedDeploymentContext,
  DeploymentApiProvider,
  WaitForDeploymentApi,
} from "@common/lib/deploymentContext";
export type {
  DeploymentInfo,
  DeploymentApiProviderProps,
} from "@common/lib/deploymentContext";

// These are used for some deployment-related settings pages
// that are not available in the self-hosted dashboard.
export {
  useDeploymentUrl,
  useDeploymentAuthHeader,
  useAdminKey,
} from "@common/lib/deploymentApi";
export * from "@common/lib/integrationHelpers";
export * from "@common/lib/stringifyValue";

// Re-usable elements
export * from "@common/elements/Button";
export * from "@common/elements/Callout";
export * from "@common/elements/ChartTooltip";
export * from "@common/elements/Checkbox";
export * from "@common/elements/ConvexLogo";
export * from "@common/elements/ClosePanelButton";
export * from "@common/elements/Combobox";
export * from "@common/elements/ConfirmationDialog";
export * from "@common/elements/CopyButton";
export * from "@common/elements/CopyTextButton";
export * from "@common/elements/DateRangePicker";
export * from "@common/elements/DetailPanel";
export * from "@common/elements/EmptySection";
export { PuzzlePieceIcon } from "@common/elements/icons";
export * from "@common/elements/Loading";
export * from "@common/elements/Menu";
export * from "@common/elements/Modal";
export * from "@common/elements/MultiSelectCombobox";
export * from "@common/elements/PageContent";
export * from "@common/elements/Popover";
export * from "@common/elements/ReadonlyCode";
export * from "@common/elements/Sheet";
export { SidebarLink, sidebarLinkClassNames } from "@common/elements/Sidebar";
export * from "@common/elements/Snippet";
export * from "@common/elements/Spinner";
export * from "@common/elements/TextInput";
export * from "@common/elements/ToggleTheme";
export * from "@common/elements/TimestampDistance";
export * from "@common/elements/Tooltip";

// For rendering the dashboard itself -- consider refactoring into a basic provider
export * from "@common/elements/Favicon";
export * from "@common/elements/ToastContainer";
export * from "@common/elements/ThemeConsumer";

// For rendering deployment pages in self-hosted dashboard
export * from "@common/layouts/DeploymentDashboardLayout";
export * from "@common/layouts/DeploymentSettingsLayout";
export * from "@common/features/health/components/HealthView";
export * from "@common/features/data/components/DataView";
export * from "@common/features/functions/components/FunctionsView";
export * from "@common/features/files/components/FileStorageView";
export * from "@common/features/logs/components/LogsView";
export * from "@common/features/history/components/HistoryView";
export * from "@common/features/schedules/components/ScheduledFunctionsView";
export * from "@common/features/settings/components/EnvironmentVariablesView";
export * from "@common/features/settings/components/AuthenticationView";
export * from "@common/features/settings/components/ComponentsView";

export * from "@common/features/settings/components/DeploymentUrl";
export * from "@common/features/settings/components/EnvironmentVariables";
export * from "@common/features/settings/components/DeploymentEnvironmentVariables";
export * from "@common/features/settings/lib/types";
