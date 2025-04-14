import {
  formatBytes,
  formatNumber,
  formatNumberCompact,
} from "@common/lib/format";

export type QuantityType =
  // Receives data in units of the given entity name
  | "unit"
  // Receives data in bytes, displays them using the appropriate data size unit
  | "storage"
  // Receives and displays data in GB * hours
  | "actionCompute";

export function formatQuantity(value: number, quantityType: QuantityType) {
  return quantityType === "storage"
    ? formatBytes(value)
    : quantityType === "actionCompute"
      ? `${formatNumber(value)} GB-hours`
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
        `${formatNumber(value)}\u00a0GBh`
      : formatNumberCompact(value);
}
