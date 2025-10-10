import { Combobox, Option } from "@ui/Combobox";

const OPTIONS: Option<number>[] = [
  { label: "Function Calls", value: 0 },
  { label: "Database Bandwidth", value: 1 },
  { label: "Action Compute", value: 2 },
  { label: "Vector Bandwidth", value: 3 },
];

export function FunctionBreakdownSelector({
  value,
  onChange,
}: {
  value: number;
  onChange: (newValue: number) => void;
}) {
  return (
    <Combobox
      label="Function Breakdown"
      labelHidden
      options={OPTIONS}
      selectedOption={value}
      setSelectedOption={(newValue) => {
        if (newValue !== null) {
          onChange(newValue);
        }
      }}
      disableSearch
      buttonClasses="w-fit"
      optionsWidth="fit"
    />
  );
}
