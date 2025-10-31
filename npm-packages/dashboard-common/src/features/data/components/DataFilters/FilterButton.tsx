import { ChevronDownIcon, MixerHorizontalIcon } from "@radix-ui/react-icons";
import { FilterExpression } from "system-udfs/convex/_system/frontend/lib/filters";
import { Button } from "@ui/Button";
import { cn } from "@ui/cn";

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
            (filter) => filter.field !== undefined && filter.enabled !== false,
          )
          .map((filter) => filter.field),
      )
    : new Set([]);
  const indexFiltersCount = countIndexFilters(filters);

  const regularFilters = filters?.clauses.filter(
    (filter) => filter.enabled !== false,
  );

  const hasAnyEnabledFilters = indexFiltersCount || validFilterNames.size > 0;

  const filterButtonContent = (
    <div className="flex items-center gap-2">
      <span>Filter & Sort</span>
      {hasAnyEnabledFilters && (
        <span className="rounded-full border border-content-primary px-1 py-0 text-xs leading-[14px] tabular-nums">
          {indexFiltersCount + (regularFilters?.length || 0)}
        </span>
      )}
    </div>
  );

  return (
    <Button
      size="xs"
      variant="neutral"
      onClick={onClick}
      aria-controls={filterMenuId}
      aria-expanded={open}
      aria-haspopup="menu"
      aria-label="Filter"
      icon={<MixerHorizontalIcon className="size-3.5" />}
      focused={open}
      className={cn(
        "w-fit rounded-l-none border-0 border-l text-xs",
        hasAnyEnabledFilters &&
          "bg-blue-100/50 hover:bg-blue-100/70 dark:bg-blue-700/50 dark:hover:bg-blue-700/70",
        // This extra padding allows other buttons aligned with the filter button to have some spacing between the
        // panel that shows when the Filter & Sort panel is open
        open && "rounded-b-none py-2.5",
      )}
      inline
    >
      {filterButtonContent}
      <ChevronDownIcon
        className={cn("transition-all", open && "-rotate-180")}
      />
    </Button>
  );
}

function countIndexFilters(filters?: FilterExpression) {
  if (filters === undefined) return 0;

  if (!filters.index) return 0;

  return (
    filters.index.clauses.filter((c) => c.enabled).length +
    // the search filter always counts as one
    ("search" in filters.index ? 1 : 0)
  );
}
