import { SegmentedControl, SegmentedControlOption } from "@ui/SegmentedControl";

// Base type for most sections (no byTable)
export type GroupBy = "byType" | "byProject";

// Extended type for Database section (includes byTable)
export type DatabaseGroupBy = GroupBy | "byTable";

// Extended type for business sections (includes byDeploymentClass)
export type BusinessGroupBy = GroupBy | "byDeploymentClass";

// Options for base GroupBy (byType, byProject)
export const GROUP_BY_OPTIONS: SegmentedControlOption<GroupBy>[] = [
  { label: "Type", value: "byType" },
  { label: "Project", value: "byProject" },
];

// Options for DatabaseGroupBy (byType, byProject, byTable)
export const DATABASE_GROUP_BY_OPTIONS: SegmentedControlOption<DatabaseGroupBy>[] =
  [
    { label: "Type", value: "byType" },
    { label: "Project", value: "byProject" },
    { label: "Table", value: "byTable" },
  ];

// Options for BusinessGroupBy (byType, byProject, byDeploymentClass)
export const BUSINESS_GROUP_BY_OPTIONS: SegmentedControlOption<BusinessGroupBy>[] =
  [
    { label: "Type", value: "byType" },
    { label: "Project", value: "byProject" },
    { label: "Deployment class", value: "byDeploymentClass" },
  ];

export function GroupBySelector<T extends string>({
  value,
  onChange,
  options,
}: {
  value: T;
  onChange: (newValue: T) => void;
  disabled?: boolean;
  options: SegmentedControlOption<T>[];
}) {
  return (
    <SegmentedControl options={options} value={value} onChange={onChange} />
  );
}
