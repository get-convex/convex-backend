import {
  ChevronUpIcon,
  ChevronDownIcon,
  MixerHorizontalIcon,
} from "@radix-ui/react-icons";
import { FilterExpression } from "system-udfs/convex/_system/frontend/lib/filters";
import { Button } from "@common/elements/Button";
import { cn } from "@common/lib/cn";

export const filterMenuId = "filterMenu";

export function FilterButton({
  filters,
  onClick,
  open,
}: {
  filters?: FilterExpression;
  onClick(): void;
  open: boolean;
}) {
  const validFilterNames = filters
    ? new Set(
        filters.clauses
          .filter(
            (filter) =>
              filter.field !== undefined && filter.value !== undefined,
          )
          .map((filter) => filter.field),
      )
    : new Set([]);
  const filterButtonContent =
    validFilterNames.size === 0 ? (
      "Filter"
    ) : (
      <div className="max-w-[9rem] truncate">
        Filtered by:{" "}
        {validFilterNames.size > 1 ? (
          `${validFilterNames.size} fields`
        ) : (
          <code>{Array.from(validFilterNames).join(", ")}</code>
        )}
      </div>
    );

  return (
    <Button
      size="xs"
      variant="neutral"
      onClick={onClick}
      aria-controls={filterMenuId}
      icon={<MixerHorizontalIcon className="size-3.5" />}
      focused={open}
      className={cn(
        "w-fit rounded-l-none text-xs border",
        open && "rounded-b-none border-b-0",
      )}
      inline
    >
      {filterButtonContent}
      {open ? <ChevronUpIcon /> : <ChevronDownIcon />}
    </Button>
  );
}
