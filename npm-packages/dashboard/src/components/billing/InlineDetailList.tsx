import { QuantityType, formatQuantity } from "./lib/formatQuantity";

export interface InlineDetailItem {
  name: string;
  value: number;
  /** Used for sort order when provided (e.g. total-range value). Falls back to value. */
  sortValue?: number;
  color: string;
}

export function InlineDetailList({
  items,
  quantityType,
}: {
  items: InlineDetailItem[];
  quantityType: QuantityType;
}) {
  const total = items.reduce((sum, item) => sum + item.value, 0);
  const sortedItems = items
    .filter((item) => (item.sortValue ?? item.value) > 0)
    .sort((a, b) => (b.sortValue ?? b.value) - (a.sortValue ?? a.value));

  return (
    <div className="flex flex-col gap-1">
      {sortedItems.map((item, index) => {
        const percentage = total > 0 ? (item.value / total) * 100 : 0;
        return (
          <div
            key={index}
            className="flex items-center justify-between gap-4 rounded-sm px-2 py-1 text-sm hover:bg-slate-900/5 dark:hover:bg-white/5"
          >
            <span className="flex items-center gap-2">
              <span
                className="size-2.5 shrink-0 rounded-full"
                style={{
                  backgroundColor: `var(--color-${item.color.replace("fill-", "")})`,
                }}
              />
              <span className="truncate">{item.name}</span>
            </span>
            <span className="flex shrink-0 items-center gap-3 tabular-nums">
              <span>{formatQuantity(item.value, quantityType)}</span>
              {sortedItems.length > 1 && (
                <span className="w-12 text-right opacity-70">
                  {percentage.toFixed(1)}%
                </span>
              )}
            </span>
          </div>
        );
      })}
    </div>
  );
}
