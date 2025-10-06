import { useMemo } from "react";
import { TextInput } from "@ui/TextInput";
import { UdfLog } from "@common/lib/useLogs";

interface SearchLogsInputProps {
  value: string;
  onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  logs?: UdfLog[];
  placeholder?: string;
}

export function SearchLogsInput({
  value,
  onChange,
  logs,
  placeholder = "Filter logs...",
}: SearchLogsInputProps) {
  // Check if the current filter matches a request ID in the logs
  const isFilterRequestId = useMemo(() => {
    if (!value || !logs || logs.length === 0) return false;
    return logs.some((log) => log.requestId === value);
  }, [value, logs]);

  return (
    <TextInput
      id="Search logs"
      placeholder={placeholder}
      value={value}
      onChange={onChange}
      type="search"
      leftAddon={
        isFilterRequestId && value ? (
          <span className="text-sm text-content-secondary">Request ID:</span>
        ) : undefined
      }
    />
  );
}
