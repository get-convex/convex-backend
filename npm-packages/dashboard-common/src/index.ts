// General dashboard usage
export {
  reportHttpError,
  copyTextToClipboard,
  toast,
  dismissToast,
  backoffWithJitter,
} from "./lib/utils";
export * from "./lib/fetching";
export * from "./lib/useGlobalLocalStorage";
export * from "./lib/useCopy";
export * from "./lib/useIsOverflowing";

// Deployment-related exports for Insights
export { useLogDeploymentEvent, useFunctionUrl } from "./lib/deploymentApi";
export {
  functionIdentifierFromValue,
  functionIdentifierValue,
} from "./lib/functions/generateFileTree";
export { useDeploymentAuditLogs } from "./lib/useDeploymentAuditLog";
export {
  useModuleFunctions,
  itemIdentifier,
} from "./lib/functions/FunctionsProvider";
export { documentHref } from "./lib/utils";
export * from "./elements/FunctionNameOption";
export * from "./elements/HealthCard";

// Re-used in the cloud dashboard for deployment pages
export {
  PROVISION_PROD_PAGE_NAME,
  DeploymentInfoContext,
  ConnectedDeploymentContext,
  DeploymentApiProvider,
  WaitForDeploymentApi,
} from "./lib/deploymentContext";
export type {
  DeploymentInfo,
  DeploymentApiProviderProps,
} from "./lib/deploymentContext";

// Used only for page headers - refactor later
export { useTableMetadataAndUpdateURL } from "./lib/useTableMetadata";

// TODO: Remove these exports once pages are refactored
export {
  useDeploymentUrl,
  useDeploymentAuthHeader,
  useAdminKey,
} from "./lib/deploymentApi";
export * from "./lib/integrationHelpers";
export * from "./lib/useNents";
export * from "./lib/stringifyValue";
export * from "./lib/format";
export * from "./lib/mockConvexReactClient";
export * from "./elements/CopyTextButton";

// Re-usable elements
export * from "./elements/Button";
export * from "./elements/Callout";
export * from "./elements/ChartTooltip";
export * from "./elements/Checkbox";
export * from "./elements/ClosePanelButton";
export * from "./elements/Combobox";
export * from "./elements/ConfirmationDialog";
export * from "./elements/CopyButton";
export * from "./elements/DateRangePicker";
export * from "./elements/DetailPanel";
export * from "./elements/EmptySection";
export { PuzzlePieceIcon } from "./elements/icons";
export * from "./elements/Loading";
export * from "./elements/Menu";
export * from "./elements/Modal";
export * from "./elements/MultiSelectCombobox";
export * from "./elements/PageContent";
export * from "./elements/Popover";
export * from "./elements/ReadonlyCode";
export * from "./elements/Sheet";
export { SidebarLink, sidebarLinkClassNames } from "./elements/Sidebar";
export * from "./elements/Snippet";
export * from "./elements/Spinner";
export * from "./elements/TextInput";
export * from "./elements/TimestampDistance";
export * from "./elements/Tooltip";

// For rendering the dashboard itself -- consider refactoring into a basic provider
export * from "./elements/Favicon";
export * from "./elements/ToastContainer";
export * from "./elements/ThemeConsumer";

// For rendering deployment pages in self-hosted dashboard
export * from "./layouts/DeploymentDashboardLayout";
export * from "./features/health/components/Health";
export * from "./features/health/components/DeploymentTimes";
export * from "./features/data/components/Data";
export * from "./features/functions/components/FunctionsView";
export * from "./features/files/components/FileStorageContent";
export * from "./features/logs/components/LogsView";
export * from "./features/history/components/History";
export * from "./features/schedules/components/ScheduledFunctionsView";
export * from "./features/schedules/components/crons/CronsView";
