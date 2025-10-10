import { Combobox, Option } from "@ui/Combobox";

export type GroupBy = "byType" | "byProject";

const OPTIONS: Option<GroupBy>[] = [
  { label: "Group by type", value: "byType" },
  { label: "Group by project", value: "byProject" },
];

export function GroupBySelector({
  value,
  onChange,
  disabled = false,
}: {
  value: GroupBy;
  onChange: (newValue: GroupBy) => void;
  disabled?: boolean;
}) {
  return (
    <Combobox
      label="Group by"
      labelHidden
      options={OPTIONS}
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
