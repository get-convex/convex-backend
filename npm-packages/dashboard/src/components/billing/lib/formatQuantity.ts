import { formatBytes, formatNumberCompact } from "@common/lib/format";

export type QuantityType =
  // Receives data in units of the given entity name
  | "unit"
  // Receives data in bytes, displays them using the appropriate data size unit
  | "storage"
  // Receives and displays data in GB * hours
  | "actionCompute";

const ACTION_COMPUTE_FORMAT_DECIMAL = new Intl.NumberFormat("en-US", {
  minimumFractionDigits: 0,
  maximumFractionDigits: 5,
});

const ACTION_COMPUTE_FORMAT_INTEGER = new Intl.NumberFormat("en-US", {
  minimumFractionDigits: 0,
  maximumFractionDigits: 0,
});

function formatActionCompute(value: number): string {
  // Show decimals only for values less than 1
  const formatter =
    value < 1 ? ACTION_COMPUTE_FORMAT_DECIMAL : ACTION_COMPUTE_FORMAT_INTEGER;
  return formatter.format(value);
}

export function formatQuantity(value: number, quantityType: QuantityType) {
  return quantityType === "storage"
    ? formatBytes(value)
    : quantityType === "actionCompute"
      ? `${formatActionCompute(value)} GB-hours`
      : formatNumberCompact(value);
}

export function formatQuantityCompact(
  value: number,
  quantityType: QuantityType,
) {
  return quantityType === "storage"
    ? formatBytes(value)
    : quantityType === "actionCompute"
      ? // non-breaking space to keep this on a single line
        `${formatActionCompute(value)}\u00a0GBh`
      : formatNumberCompact(value);
}
