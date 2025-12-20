import { cn } from "@ui/cn";

type DateTimePickerProps = {
  date: Date;
  onChange: (date: Date) => void;
  onSave?: () => void;
  disabled?: boolean;
  autoFocus?: boolean;
  className?: string;
};

const toDateTimeLocalValue = (d: Date) => {
  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  const hours = String(d.getHours()).padStart(2, "0");
  const minutes = String(d.getMinutes()).padStart(2, "0");
  const seconds = String(d.getSeconds()).padStart(2, "0");
  return `${year}-${month}-${day}T${hours}:${minutes}:${seconds}`;
};

export function DateTimePicker({
  date,
  onChange,
  onSave,
  disabled = false,
  autoFocus = false,
  className,
}: DateTimePickerProps) {
  return (
    <input
      autoFocus={autoFocus}
      type="datetime-local"
      className={cn(
        "focus:outline-none disabled:cursor-not-allowed disabled:bg-background-tertiary disabled:text-content-secondary",
        className,
      )}
      disabled={disabled}
      step={1}
      defaultValue={toDateTimeLocalValue(date)}
      onChange={(d) => {
        if (d.target.value) {
          onChange(new Date(d.target.value));
        }
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          onSave?.();
        }
      }}
    />
  );
}
