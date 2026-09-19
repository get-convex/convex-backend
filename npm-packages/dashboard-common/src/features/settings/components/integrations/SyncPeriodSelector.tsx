import { Combobox, Option } from "@ui/Combobox";
import { SyncPeriod } from "system-udfs/convex/_system/frontend/common";

const syncPeriodOptions: Option<SyncPeriod>[] = [
  { value: "daily", label: "Daily" },
  { value: "hourly", label: "Hourly" },
  { value: "continuous", label: "Continuous" },
];

export function SyncPeriodSelector({
  value,
  onChange,
}: {
  value: SyncPeriod;
  onChange: (period: SyncPeriod) => void;
}) {
  return (
    <div className="flex flex-col gap-1">
      <Combobox
        label="Sync Frequency"
        labelHidden={false}
        options={syncPeriodOptions}
        selectedOption={value}
        setSelectedOption={(period) => period && onChange(period)}
        allowCustomValue={false}
        buttonClasses="w-full bg-inherit"
      />
      <div className="max-w-prose text-xs text-content-secondary">
        How often the mirror is refreshed. Your data lags the deployment by up
        to this interval.
      </div>
    </div>
  );
}
