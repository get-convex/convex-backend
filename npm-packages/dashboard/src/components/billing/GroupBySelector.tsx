import { Combobox, Option } from "@ui/Combobox";

// Base type for most sections (no byTable)
export type GroupBy = "byType" | "byProject";

// Extended type for Database section (includes byTable)
export type DatabaseGroupBy = GroupBy | "byTable";

// Options for base GroupBy (byType, byProject)
export const GROUP_BY_OPTIONS: Option<GroupBy>[] = [
  { label: "Group by type", value: "byType" },
  { label: "Group by project", value: "byProject" },
];

// Options for DatabaseGroupBy (byType, byProject, byTable)
export const DATABASE_GROUP_BY_OPTIONS: Option<DatabaseGroupBy>[] = [
  { label: "Group by type", value: "byType" },
  { label: "Group by project", value: "byProject" },
  { label: "Group by table", value: "byTable" },
];

export function GroupBySelector<T>({
  value,
  onChange,
  disabled = false,
  options,
}: {
  value: T;
  onChange: (newValue: T) => void;
  disabled?: boolean;
  options: Option<T>[];
}) {
  return (
    <Combobox
      label="Group by"
      labelHidden
      options={options}
      buttonProps={{
        tip: disabled
          ? "You cannot change the grouping while filtered to a specific project."
          : undefined,
      }}
      selectedOption={value}
      setSelectedOption={(newValue) => {
        if (newValue) {
          onChange(newValue);
        }
      }}
      disableSearch
      disabled={disabled}
      buttonClasses="w-fit"
      optionsWidth="fit"
    />
  );
}
