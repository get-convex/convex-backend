/**
 * This barrel file exports all code that is shared between the
 * self-hosted dashboard and the cloud dashboard.
 */

// General dashboard exports
export {
  reportHttpError,
  toast,
  dismissToast,
  backoffWithJitter,
} from "lib/utils";
export * from "lib/fetching";
export * from "lib/useGlobalLocalStorage";
export * from "lib/useCopy";
export * from "lib/useIsOverflowing";
export {
  formatBytes,
  formatNumber,
  formatNumberCompact,
  formatDate,
  msFormat,
  toNumericUTC,
} from "lib/format";

// Deployment-related exports for Insights
export { useFunctionUrl } from "lib/deploymentApi";
export {
  functionIdentifierFromValue,
  functionIdentifierValue,
} from "lib/functions/generateFileTree";
export * from "lib/useNents";
export { useDeploymentAuditLogs } from "lib/useDeploymentAuditLog";
export {
  useModuleFunctions,
  itemIdentifier,
} from "lib/functions/FunctionsProvider";
export { documentHref } from "lib/utils";
export * from "elements/FunctionNameOption";
export * from "elements/HealthCard";
export * from "features/health/components/DeploymentTimes";

// Re-used in the cloud dashboard for deployment pages
export {
  PROVISION_PROD_PAGE_NAME,
  DeploymentInfoContext,
  ConnectedDeploymentContext,
  DeploymentApiProvider,
  WaitForDeploymentApi,
} from "lib/deploymentContext";
export type {
  DeploymentInfo,
  DeploymentApiProviderProps,
} from "lib/deploymentContext";

// These are used for some deployment-related settings pages
// that are not available in the self-hosted dashboard.
export {
  useDeploymentUrl,
  useDeploymentAuthHeader,
  useAdminKey,
} from "lib/deploymentApi";
export * from "lib/integrationHelpers";
export * from "lib/stringifyValue";

// Re-usable elements
export * from "elements/Button";
export * from "elements/Callout";
export * from "elements/ChartTooltip";
export * from "elements/Checkbox";
export * from "elements/ConvexLogo";
export * from "elements/ClosePanelButton";
export * from "elements/Combobox";
export * from "elements/ConfirmationDialog";
export * from "elements/CopyButton";
export * from "elements/CopyTextButton";
export * from "elements/DateRangePicker";
export * from "elements/DetailPanel";
export * from "elements/EmptySection";
export { PuzzlePieceIcon } from "elements/icons";
export * from "elements/Loading";
export * from "elements/Menu";
export * from "elements/Modal";
export * from "elements/MultiSelectCombobox";
export * from "elements/PageContent";
export * from "elements/Popover";
export * from "elements/ReadonlyCode";
export * from "elements/Sheet";
export { SidebarLink, sidebarLinkClassNames } from "elements/Sidebar";
export * from "elements/Snippet";
export * from "elements/Spinner";
export * from "elements/TextInput";
export * from "elements/ToggleTheme";
export * from "elements/TimestampDistance";
export * from "elements/Tooltip";

// For rendering the dashboard itself -- consider refactoring into a basic provider
export * from "elements/Favicon";
export * from "elements/ToastContainer";
export * from "elements/ThemeConsumer";

// For rendering deployment pages in self-hosted dashboard
export * from "layouts/DeploymentDashboardLayout";
export * from "layouts/DeploymentSettingsLayout";
export * from "features/health/components/HealthView";
export * from "features/data/components/DataView";
export * from "features/functions/components/FunctionsView";
export * from "features/files/components/FileStorageView";
export * from "features/logs/components/LogsView";
export * from "features/history/components/HistoryView";
export * from "features/schedules/components/ScheduledFunctionsView";
export * from "features/schedules/components/crons/CronsView";
export * from "features/settings/components/EnvironmentVariablesView";
export * from "features/settings/components/AuthenticationView";
export * from "features/settings/components/ComponentsView";

export * from "features/settings/components/DeploymentUrl";
export * from "features/settings/components/EnvironmentVariables";
export * from "features/settings/components/DeploymentEnvironmentVariables";
export * from "features/settings/lib/types";
